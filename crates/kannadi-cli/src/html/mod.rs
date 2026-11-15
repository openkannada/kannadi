//! HTML files. Only text set in a legacy font is converted to Unicode, and only
//! Kannada text outside legacy fonts is converted to Nudi. The font of each text
//! node comes from `style` attributes, `<font face>`, and `<style>` rules for tags
//! and classes, inherited from enclosing elements. The output is UTF-8.

mod css;
mod entities;
mod tokens;

use std::ops::Range;

use anyhow::Result;

use crate::convert::{convert, kannada_runs, Direction, Issue, Settings};
use crate::fonts::{first_family, replace_legacy};
use crate::report::{Report, Row};
use crate::txt::read_text;
use tokens::{tokenize, Tag, Token};

pub fn convert_file(bytes: &[u8], settings: &Settings) -> Result<(Vec<u8>, Report)> {
    let html = read_text(bytes, settings.direction)?;
    let mut walker = Walker {
        settings,
        rules: css::Rules::default(),
        stack: Vec::new(),
        out: String::with_capacity(html.len() * 2),
        report: Report::default(),
        line: 1,
        charset_declared: false,
        charset_at: None,
    };
    for token in tokenize(&html) {
        walker.token(&token);
    }
    Ok(walker.finish())
}

/// Elements with no end tag.
const VOID: [&str; 17] = [
    "area", "base", "basefont", "br", "col", "embed", "frame", "hr", "img", "input", "isindex",
    "keygen", "link", "meta", "param", "source", "wbr",
];

struct Open {
    name: String,
    legacy: bool,
}

struct Walker<'s> {
    settings: &'s Settings,
    rules: css::Rules,
    stack: Vec<Open>,
    out: String,
    report: Report,
    line: usize,
    charset_declared: bool,
    /// Where a charset declaration goes if the document has none.
    charset_at: Option<usize>,
}

impl Walker<'_> {
    fn token(&mut self, token: &Token) {
        match token {
            Token::Other(s) => self.out.push_str(s),
            Token::Raw { element, text } => {
                if *element == "style" {
                    self.rules.add(text);
                }
                if *element == "style" && self.to_unicode() {
                    let fonts = &self.settings.fonts;
                    let css = css::rewrite_font_families(text, |v| {
                        replace_legacy(v, &fonts.unicode, Some('"'), &fonts.extra_legacy)
                    });
                    self.out.push_str(&css);
                } else {
                    self.out.push_str(text);
                }
            }
            Token::Tag(tag) if tag.closing => {
                if let Some(i) = self.stack.iter().rposition(|o| o.name == tag.name) {
                    self.stack.truncate(i);
                }
                self.out.push_str(tag.source);
            }
            Token::Tag(tag) => self.open(tag),
            Token::Text(text) => self.text(text),
        }
        let source = match token {
            Token::Text(s) | Token::Other(s) => s,
            Token::Tag(t) => t.source,
            Token::Raw { text, .. } => text,
        };
        self.line += source.matches('\n').count();
    }

    fn open(&mut self, tag: &Tag) {
        self.close_implied(&tag.name);
        let fonts = &self.settings.fonts;
        let style = tag.attr("style").map(|a| a.value);
        let face = (tag.name == "font").then(|| tag.attr("face")).flatten();
        let classes = tag.attr("class").map_or("", |a| a.value);
        let family = style
            .and_then(css::font_family)
            .or(face.map(|a| a.value))
            .or_else(|| self.rules.family(&tag.name, classes));
        let legacy = match family {
            Some(f) => fonts.is_legacy(first_family(f)),
            None => self.stack.last().is_some_and(|o| o.legacy),
        };

        let mut edits: Vec<(Range<usize>, String)> = Vec::new();
        if self.to_unicode() {
            let replace = |v: &str, q| replace_legacy(v, &fonts.unicode, q, &fonts.extra_legacy);
            if let Some(a) = tag.attr("style") {
                let inner = match tag.source[..a.span.start].chars().next_back() {
                    Some('"') => '\'',
                    _ => '"',
                };
                let css = css::rewrite_font_families(a.value, |v| replace(v, Some(inner)));
                edits.push((a.span.clone(), css));
            }
            if let Some(a) = face {
                edits.push((a.span.clone(), replace(a.value, None)));
            }
        }
        if tag.name == "meta" {
            if let Some(edit) = charset_edit(tag) {
                self.charset_declared = true;
                edits.push(edit);
            }
        }
        self.out.push_str(&apply(tag.source, edits));
        if tag.name == "head" || (tag.name == "html" && self.charset_at.is_none()) {
            self.charset_at = Some(self.out.len());
        }

        let void = VOID.contains(&tag.name.as_str()) || tag.source.ends_with("/>");
        if !void {
            self.stack.push(Open {
                name: tag.name.clone(),
                legacy,
            });
        }
    }

    /// Closes elements whose end tag HTML lets authors leave out.
    fn close_implied(&mut self, name: &str) {
        let closes: &[&str] = match name {
            "p" => &["p"],
            "li" => &["li"],
            "td" | "th" => &["td", "th"],
            "tr" => &["td", "th", "tr"],
            "dt" | "dd" => &["dt", "dd"],
            "option" => &["option"],
            _ => &[],
        };
        while self
            .stack
            .last()
            .is_some_and(|o| closes.contains(&o.name.as_str()))
        {
            self.stack.pop();
        }
    }

    fn text(&mut self, source: &str) {
        let legacy = self.stack.last().is_some_and(|o| o.legacy);
        match (self.settings.direction, legacy) {
            (Direction::ToUnicode(_), true) => {
                let text = entities::decode(source);
                let converted = convert(&text, self.settings);
                self.rows(self.line, &converted.issues);
                self.out.push_str(&entities::escape(&converted.text));
            }
            (Direction::ToNudi, false) => self.wrap_kannada(source),
            _ => self.out.push_str(source),
        }
    }

    /// Wraps each Kannada run in a span set in the legacy font.
    fn wrap_kannada(&mut self, source: &str) {
        let text = entities::decode(source);
        let runs = kannada_runs(&text);
        if runs.is_empty() {
            self.out.push_str(source);
            return;
        }
        let mut last = 0;
        for run in runs {
            self.out.push_str(&entities::escape(&text[last..run.start]));
            let piece = &text[run.clone()];
            let converted = convert(piece, self.settings);
            let line = self.line + text[..run.start].matches('\n').count();
            self.rows(line, &converted.issues);
            self.out.push_str(&format!(
                "<span style=\"font-family:'{}'\">{}</span>",
                self.settings.fonts.legacy,
                entities::escape(&converted.text)
            ));
            last = run.end;
        }
        self.out.push_str(&entities::escape(&text[last..]));
    }

    fn to_unicode(&self) -> bool {
        matches!(self.settings.direction, Direction::ToUnicode(_))
    }

    /// Adds rows for the issues of a piece of text that starts on `line`.
    fn rows(&mut self, line: usize, issues: &[Issue]) {
        for issue in issues {
            self.report.rows.push(Row {
                location: format!("line {}", line + issue.line - 1),
                kind: issue.kind,
                detail: issue.detail.clone(),
                context: issue.context.clone(),
            });
        }
    }

    fn finish(mut self) -> (Vec<u8>, Report) {
        if !self.charset_declared {
            let at = self.charset_at.unwrap_or(0);
            self.out.insert_str(at, "<meta charset=\"utf-8\">");
        }
        (self.out.into_bytes(), self.report)
    }
}

/// The edit that makes a `<meta>` declare UTF-8, if it declares a charset.
fn charset_edit(tag: &Tag) -> Option<(Range<usize>, String)> {
    if let Some(a) = tag.attr("charset") {
        return Some((a.span.clone(), "utf-8".to_string()));
    }
    let a = tag.attr("content")?;
    let at = a.value.to_ascii_lowercase().find("charset=")?;
    let start = a.span.start + at + "charset=".len();
    let len = a.value[at + "charset=".len()..]
        .find(|c: char| c == ';' || c.is_whitespace())
        .unwrap_or(a.value.len() - at - "charset=".len());
    Some((start..start + len, "utf-8".to_string()))
}

/// `source` with each range replaced; ranges must not overlap.
fn apply(source: &str, mut edits: Vec<(Range<usize>, String)>) -> String {
    edits.sort_by_key(|(r, _)| r.start);
    let mut out = String::with_capacity(source.len());
    let mut last = 0;
    for (range, text) in edits {
        out.push_str(&source[last..range.start]);
        out.push_str(&text);
        last = range.end;
    }
    out.push_str(&source[last..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::Kind;
    use kannadi::legacy::Options;

    fn run(html: &str, direction: Direction) -> (String, Report) {
        let settings = Settings::new(direction);
        let (out, report) = convert_file(html.as_bytes(), &settings).unwrap();
        (String::from_utf8(out).unwrap(), report)
    }

    #[test]
    fn only_legacy_font_text_becomes_unicode() {
        let html = "<html><head><style>.k {font-family:\"BRH Kannada\"}</style></head>\
                    <body><p class=k>PÀ£ÀßqÀ <b>PÀ</b></p><p>PÀ</p>\
                    <font face=\"Nudi 01 e\">£À</font><span style=\"font-family:BRH Kasturi\">qÀ</span></body></html>";
        let (out, report) = run(html, Direction::ToUnicode(Options::default()));
        assert!(
            out.contains("<p class=k>ಕನ್ನಡ <b>ಕ</b></p><p>PÀ</p>"),
            "{out}"
        );
        assert!(
            out.contains("<font face=\"Noto Sans Kannada\">ನ</font>"),
            "{out}"
        );
        assert!(
            out.contains("<span style=\"font-family:'Noto Sans Kannada'\">ಡ</span>"),
            "{out}"
        );
        assert!(
            out.contains(".k {font-family:\"Noto Sans Kannada\"}"),
            "{out}"
        );
        assert!(
            out.starts_with("<html><head><meta charset=\"utf-8\"><style>"),
            "{out}"
        );
        assert!(report.is_empty());
    }

    #[test]
    fn entities_and_windows_1252_bytes_decode() {
        let html =
            b"<meta http-equiv=\"Content-Type\" content=\"text/html; charset=windows-1252\">\
                     <p style=\"font-family:'BRH Kannada'\">P&Agrave;&#163;\xC0 &amp;</p>";
        let settings = Settings::new(Direction::ToUnicode(Options::default()));
        let (out, _) = convert_file(html, &settings).unwrap();
        let out = String::from_utf8(out).unwrap();
        assert!(out.contains("charset=utf-8\">"), "{out}");
        assert!(out.contains(">ಕನ &amp;</p>"), "{out}");
    }

    #[test]
    fn unclosed_cells_do_not_leak_their_font() {
        let html =
            "<style>td.k{font-family:BRH Kannada}</style><table><tr><td class=k>PÀ<td>PÀ</table>";
        let (out, _) = run(html, Direction::ToUnicode(Options::default()));
        assert!(out.contains("<td class=k>ಕ<td>PÀ"), "{out}");
    }

    #[test]
    fn void_elements_do_not_pass_on_their_font() {
        let html = "<p><img style=\"font-family:BRH Kannada\">PÀ<br class=k>PÀ</p>";
        let (out, _) = run(html, Direction::ToUnicode(Options::default()));
        assert!(out.ends_with("\">PÀ<br class=k>PÀ</p>"), "{out}");
    }

    #[test]
    fn reports_name_the_line() {
        let html = "<p style=\"font-family:Nudi 01 e\">\nPÀ\n\u{F1}</p>";
        let (_, report) = run(html, Direction::ToUnicode(Options::default()));
        assert_eq!(report.rows[0].kind, Kind::UnknownGlyph);
        assert_eq!(report.rows[0].location, "line 3");
    }

    #[test]
    fn kannada_text_gets_a_legacy_span() {
        let html = "<p>Office: ಕನ್ನಡ &amp; ನಾಡು</p><script>var s = 'ಕ';</script>";
        let (out, report) = run(html, Direction::ToNudi);
        assert!(
            out.contains(
                "<p>Office: <span style=\"font-family:'Nudi 01 e'\">PÀ£ÀßqÀ &amp; £ÁqÀÄ</span></p>"
            ),
            "{out}"
        );
        assert!(out.contains("<script>var s = 'ಕ';</script>"), "{out}");
        assert!(report.is_empty());
    }

    #[test]
    fn legacy_text_stays_when_converting_to_nudi() {
        let html = "<p style=\"font-family:BRH Kannada\">PÀ</p>";
        let (out, _) = run(html, Direction::ToNudi);
        assert!(
            out.ends_with("<p style=\"font-family:BRH Kannada\">PÀ</p>"),
            "{out}"
        );
    }
}
