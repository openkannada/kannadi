//! HTML files. Only text set in a legacy font is converted to Unicode, and only
//! Kannada text outside legacy fonts is converted to Nudi. The font of each text
//! node comes from its `style` attribute, then `<style>` rules for tags and
//! classes, then `<font face>`, inherited from enclosing elements. Text in
//! `<title>`, `<textarea>` and attributes is left as it is. The output is UTF-8.

mod css;
mod entities;
mod tokens;

use std::ops::Range;

use anyhow::Result;
use kannadi::legacy::{decode_windows_1252, Layout};

use crate::convert::{convert, kannada_runs, Direction, Issue, Settings};
use crate::edits::apply;
use crate::fonts::first_family;
use crate::report::{Report, Row};
use crate::txt::read_text;
use tokens::{tokenize, Attr, RawKind, Tag, Token};

pub fn convert_file(bytes: &[u8], settings: &Settings) -> Result<(Vec<u8>, Report)> {
    let bom = [&b"\xEF\xBB\xBF"[..], b"\xFF\xFE", b"\xFE\xFF"]
        .iter()
        .any(|m| bytes.starts_with(m));
    let html = match declared_charset(bytes) {
        Some(Charset::Windows1252) if !bom => decode_windows_1252(bytes),
        _ => read_text(bytes, settings.direction)?,
    };
    let mut walker = Walker {
        settings,
        rules: css::Rules::default(),
        stack: Vec::new(),
        out: String::with_capacity(html.len() * 2),
        report: Report::default(),
        line: 1,
        charset_declared: false,
        charset_at: None,
        prologue_end: 0,
        in_prologue: true,
    };
    for token in tokenize(&html) {
        walker.token(&token);
    }
    Ok(walker.finish())
}

#[derive(Debug, PartialEq, Eq)]
enum Charset {
    Windows1252,
    Other,
}

/// The charset a `<meta>` in the first 1024 bytes declares, ignoring comments and
/// scripts. Browsers read ISO-8859-1 and US-ASCII pages as Windows-1252.
fn declared_charset(bytes: &[u8]) -> Option<Charset> {
    let head = String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]);
    let name = tokenize(&head).into_iter().find_map(|token| match token {
        Token::Tag(tag) if tag.name == "meta" && !tag.closing => {
            if let Some(a) = tag.attr("charset") {
                return Some(a.value.trim().to_ascii_lowercase());
            }
            let content = tag.attr("content")?.value.to_ascii_lowercase();
            let at = content.find("charset=")? + "charset=".len();
            let name = content[at..].split([';', ' ']).next().unwrap_or_default();
            Some(name.trim().trim_matches(['"', '\'']).to_string())
        }
        _ => None,
    })?;
    let latin = ["windows-1252", "iso-8859-1", "latin1", "us-ascii", "cp1252"];
    Some(if latin.contains(&name.as_str()) {
        Charset::Windows1252
    } else {
        Charset::Other
    })
}

/// Elements with no end tag.
const VOID: [&str; 17] = [
    "area", "base", "basefont", "br", "col", "embed", "frame", "hr", "img", "input", "isindex",
    "keygen", "link", "meta", "param", "source", "wbr",
];

/// Elements whose start closes an open `<p>`.
const CLOSES_P: [&str; 25] = [
    "address",
    "article",
    "aside",
    "blockquote",
    "center",
    "dir",
    "div",
    "dl",
    "fieldset",
    "figure",
    "footer",
    "form",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "header",
    "hr",
    "menu",
    "ol",
    "p",
    "pre",
    "table",
];

struct Open {
    name: String,
    /// The layout of text in the element, if it is set in a legacy font.
    layout: Option<Layout>,
}

struct Walker<'s> {
    settings: &'s Settings,
    rules: css::Rules,
    stack: Vec<Open>,
    out: String,
    report: Report,
    line: usize,
    charset_declared: bool,
    /// Just after `<head>`, or `<html>` if there is no head.
    charset_at: Option<usize>,
    /// Just after the leading doctype and comments.
    prologue_end: usize,
    in_prologue: bool,
}

impl Walker<'_> {
    fn token(&mut self, token: &Token) {
        match token {
            Token::Other(s) => self.out.push_str(s),
            Token::Raw { kind, text } => self.raw(*kind, text),
            Token::Tag(tag) if tag.closing => {
                if let Some(i) = self.stack.iter().rposition(|o| o.name == tag.name) {
                    self.stack.truncate(i);
                }
                self.out.push_str(tag.source);
            }
            Token::Tag(tag) => self.open(tag),
            Token::Text(text) => self.text(text),
        }
        if self.in_prologue {
            self.in_prologue = match token {
                Token::Other(_) => true,
                Token::Text(t) => t.trim().is_empty(),
                _ => false,
            };
            if self.in_prologue {
                self.prologue_end = self.out.len();
            }
        }
        self.line += token.source().matches('\n').count();
    }

    fn raw(&mut self, kind: RawKind, text: &str) {
        if kind == RawKind::Style {
            self.rules.add(text);
            if self.to_unicode() {
                let css =
                    css::rewrite_font_families(text, true, |v| self.unicode_family(v, Some('"')));
                self.out.push_str(&css);
                return;
            }
        }
        self.out.push_str(text);
    }

    fn open(&mut self, tag: &Tag) {
        self.close_implied(&tag.name);
        let decoded = |a: &Attr| entities::decode(a.value).text;
        let style = tag.attr("style").map(decoded);
        let face = (tag.name == "font")
            .then(|| tag.attr("face").map(decoded))
            .flatten();
        let classes = tag.attr("class").map(decoded).unwrap_or_default();
        let family = style
            .as_deref()
            .and_then(css::font_family)
            .or_else(|| self.rules.family(&tag.name, &classes).map(str::to_string))
            .or(face.clone());
        let inherited = self.stack.last().and_then(|o| o.layout);
        let layout = match family.as_deref().map(css::Family::of) {
            Some(css::Family::Named(f)) => self.settings.fonts.layout(first_family(f)),
            Some(css::Family::Initial) => None,
            Some(css::Family::Inherit) | None => inherited,
        };

        let mut edits: Vec<(Range<usize>, String)> = Vec::new();
        if self.to_unicode() {
            if let (Some(a), Some(value)) = (tag.attr("style"), &style) {
                let inner = if a.quote == Some('"') { '\'' } else { '"' };
                let css = css::rewrite_font_families(value, false, |v| {
                    self.unicode_family(v, Some(inner))
                });
                if css != *value {
                    edits.push(attribute_edit(a, &css));
                }
            }
            if let (Some(a), Some(value)) = (tag.attr("face"), &face) {
                let new = self.unicode_family(value, None);
                if new != *value {
                    edits.push(attribute_edit(a, &new));
                }
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
        if !VOID.contains(&tag.name.as_str()) {
            self.stack.push(Open {
                name: tag.name.clone(),
                layout,
            });
        }
    }

    /// Closes elements whose end tag HTML lets authors leave out: up to and
    /// including the nearest one the new element ends, if no boundary comes first.
    fn close_implied(&mut self, name: &str) {
        let (ends, boundary): (&[&str], &[&str]) = match name {
            "li" => (&["li"], &["ul", "ol", "menu", "dir"]),
            "dt" | "dd" => (&["dt", "dd"], &["dl"]),
            "td" | "th" => (&["td", "th"], &["table", "tr"]),
            "tr" => (&["tr"], &["table", "thead", "tbody", "tfoot"]),
            "option" => (&["option"], &["select", "datalist"]),
            n if CLOSES_P.contains(&n) => (&["p"], &["table", "td", "th", "button", "li"]),
            _ => return,
        };
        for i in (0..self.stack.len()).rev() {
            let open = self.stack[i].name.as_str();
            if ends.contains(&open) {
                self.stack.truncate(i);
                return;
            }
            if boundary.contains(&open) {
                return;
            }
        }
    }

    fn text(&mut self, source: &str) {
        let layout = self.stack.last().and_then(|o| o.layout);
        match (self.settings.direction, layout) {
            (Direction::ToUnicode(_), Some(layout)) => {
                let text = entities::decode(source).text;
                let converted = convert(&text, layout, self.settings);
                self.rows(self.line, &converted.issues);
                self.out.push_str(&entities::escape(&converted.text, None));
            }
            (Direction::ToNudi, None) => self.wrap_kannada(source),
            _ => self.out.push_str(source),
        }
    }

    /// Wraps each Kannada run in a span set in the legacy font. Text between runs
    /// is copied from the source as it was.
    fn wrap_kannada(&mut self, source: &str) {
        let decoded = entities::decode(source);
        let font = entities::escape(&self.settings.fonts.legacy, Some('\'')).replace('"', "&quot;");
        let mut last = 0;
        for run in kannada_runs(&decoded.text) {
            let start = decoded.source_offset(run.start);
            self.out.push_str(&source[last..start]);
            let converted = convert(
                &decoded.text[run.clone()],
                self.settings.layout,
                self.settings,
            );
            let line = self.line + source[..start].matches('\n').count();
            self.rows(line, &converted.issues);
            self.out.push_str(&format!(
                "<span style=\"font-family:'{font}'\">{}</span>",
                entities::escape(&converted.text, None)
            ));
            last = decoded.source_offset(run.end);
        }
        self.out.push_str(&source[last..]);
    }

    fn to_unicode(&self) -> bool {
        matches!(self.settings.direction, Direction::ToUnicode(_))
    }

    /// A `font-family` list with legacy fonts replaced by the Unicode font.
    fn unicode_family(&self, list: &str, quote: Option<char>) -> String {
        self.settings.fonts.unicode_family(list, quote)
    }

    /// Adds rows for the issues of a piece of text that starts on `line`.
    fn rows(&mut self, line: usize, issues: &[Issue]) {
        for issue in issues {
            let location = format!("line {}", line + issue.line - 1);
            self.report.rows.push(Row::new(location, issue));
        }
    }

    fn finish(mut self) -> (Vec<u8>, Report) {
        if !self.charset_declared {
            let at = self.charset_at.unwrap_or(self.prologue_end);
            self.out.insert_str(at, "<meta charset=\"utf-8\">");
        }
        (self.out.into_bytes(), self.report)
    }
}

/// The edit that sets an attribute's value, quoting and escaping it as needed.
fn attribute_edit(a: &Attr, value: &str) -> (Range<usize>, String) {
    match a.quote {
        Some(q) => (a.span.clone(), entities::escape(value, Some(q))),
        None => (
            a.span.clone(),
            format!("\"{}\"", entities::escape(value, Some('"'))),
        ),
    }
}

/// The edit that makes a `<meta>` declare UTF-8, if it declares a charset.
fn charset_edit(tag: &Tag) -> Option<(Range<usize>, String)> {
    if let Some(a) = tag.attr("charset") {
        let bare = !a.assigned;
        let value = if bare { "=\"utf-8\"" } else { "utf-8" };
        return Some((a.span.clone(), value.to_string()));
    }
    let a = tag.attr("content")?;
    let at = a.value.to_ascii_lowercase().find("charset=")? + "charset=".len();
    let len = a.value[at..]
        .find(|c: char| c == ';' || c.is_whitespace())
        .unwrap_or(a.value.len() - at);
    let start = a.span.start + at;
    Some((start..start + len, "utf-8".to_string()))
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

    fn to_unicode(html: &str) -> String {
        run(html, Direction::ToUnicode(Options::default())).0
    }

    #[test]
    fn each_font_is_read_in_its_layout() {
        let html = "<p><span style=\"font-family:AkrutiKndPadmaDemi\">d&#226;}&#226;</span>\
                    <font face=\"Nudi 01 e\">PÀ</font></p>";
        let out = to_unicode(html);
        assert!(
            out.contains("<span style=\"font-family:'Noto Sans Kannada'\">ಕನ</span>"),
            "{out}"
        );
        assert!(
            out.contains("<font face=\"Noto Sans Kannada\">ಕ</font>"),
            "{out}"
        );
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
                     <p style=\"font-family:'BRH Kannada'\">P&Agrave;&#163;\xC0 &amp;</p>\
                     <p style=\"font-family:BRH Kannada\">\xC4\xB8</p>";
        let settings = Settings::new(Direction::ToUnicode(Options::default()));
        let (out, report) = convert_file(html, &settings).unwrap();
        let out = String::from_utf8(out).unwrap();
        assert!(out.contains("charset=utf-8\">"), "{out}");
        assert!(out.contains(">ಕನ &amp;</p>"), "{out}");
        assert!(!out.contains('\u{0138}'), "{out}");
        assert_eq!(report.rows.len(), 2, "{:?}", report.rows);
    }

    #[test]
    fn a_declared_windows_1252_charset_wins_over_valid_utf8() {
        // Valid UTF-8 for "Qä", and the glyph codes of ಕೀನಿ in Windows-1252.
        let html = b"<meta charset=windows-1252><p style=\"font-family:BRH Kannada\">Q\xC3\xA4</p>";
        let settings = Settings::new(Direction::ToUnicode(Options::default()));
        let out = String::from_utf8(convert_file(html, &settings).unwrap().0).unwrap();
        assert!(out.contains(">ಕೀನಿ</p>"), "{out}");
    }

    #[test]
    fn unclosed_elements_do_not_leak_their_font() {
        let cells =
            "<style>td.k{font-family:BRH Kannada}</style><table><tr><td class=k>PÀ<td>PÀ</table>";
        assert!(to_unicode(cells).contains("<td class=k>ಕ<td>PÀ"));
        let font = "<table><tr><td><font face=\"Nudi 01 e\">PÀ<td>Hello</table>";
        assert!(to_unicode(font).contains("<td>Hello"));
        let p = "<p style=\"font-family:Nudi 01 e\">PÀ<div>English</div>";
        assert!(to_unicode(p).contains("<div>English</div>"));
        let li = "<ul><li><font face=\"Nudi 01 e\">PÀ<li>Hello</ul>";
        assert!(to_unicode(li).contains("<li>Hello"));
    }

    #[test]
    fn void_elements_do_not_pass_on_their_font() {
        let out = to_unicode("<p><img style=\"font-family:BRH Kannada\">PÀ<br class=k>PÀ</p>");
        assert!(out.ends_with("\">PÀ<br class=k>PÀ</p>"), "{out}");
    }

    #[test]
    fn style_wins_over_rules_and_rules_over_face() {
        let html = "<style>font{font-family:Arial}</style><font face=\"Nudi 01 e\">English</font>\
                    <p style=\"font-family:&quot;BRH Kannada&quot;\">PÀ</p>\
                    <div style=\"font-family:BRH Kannada\"><span style=\"font-family:inherit\">PÀ</span></div>";
        let out = to_unicode(html);
        assert!(out.contains(">English</font>"), "{out}");
        assert!(
            out.contains("<p style=\"font-family:'Noto Sans Kannada'\">ಕ</p>"),
            "{out}"
        );
        assert!(
            out.contains("<span style=\"font-family:inherit\">ಕ</span>"),
            "{out}"
        );
    }

    #[test]
    fn rewritten_attributes_stay_valid() {
        let out = to_unicode("<font face=Nudi01e class=x>PÀ</font><p style=font-family:BRH>PÀ</p>");
        assert!(
            out.contains("<font face=\"Noto Sans Kannada\" class=x>ಕ</font>"),
            "{out}"
        );
        assert!(
            out.contains("<p style=\"font-family:&quot;Noto Sans Kannada&quot;\">ಕ</p>"),
            "{out}"
        );
        let important = to_unicode("<p style='font-family:BRH Kannada !important'>PÀ</p>");
        assert!(
            important.contains("font-family:\"Noto Sans Kannada\" !important'>ಕ"),
            "{important}"
        );
    }

    #[test]
    fn charset_comes_from_real_meta_tags_and_a_bom_wins() {
        let html = "<!-- charset=windows-1252 --><meta charset=utf-8><p style=\"font-family:BRH Kannada\">PÀ ಕ</p>";
        assert!(to_unicode(html).contains(">ಕ ಕ</p>"));
        let bom = "\u{FEFF}<meta charset=windows-1252><p style=\"font-family:BRH Kannada\">PÀ</p>";
        assert!(to_unicode(bom).contains(">ಕ</p>"));
        let quoted =
            b"<meta http-equiv=\"Content-Type\" content=\"text/html; charset='windows-1252'\">\
                       <p style=\"font-family:BRH Kannada\">Q\xC3\xA4</p>";
        let settings = Settings::new(Direction::ToUnicode(Options::default()));
        let out = String::from_utf8(convert_file(quoted, &settings).unwrap().0).unwrap();
        assert!(out.contains(">ಕೀನಿ</p>"), "{out}");
    }

    #[test]
    fn initial_resets_the_font_and_comments_are_skipped() {
        let html = "<style>.k{/* note */font-family:BRH Kannada}</style><p class=k>PÀ</p>\
                    <div style=\"font-family:BRH Kannada\"><span style=\"font-family:initial\">English</span></div>";
        let out = to_unicode(html);
        assert!(out.contains("<p class=k>ಕ</p>"), "{out}");
        assert!(out.contains(">English</span>"), "{out}");
    }

    #[test]
    fn charset_declarations_are_kept_valid() {
        let doctype = to_unicode("<!DOCTYPE html><p>x</p>");
        assert!(
            doctype.starts_with("<!DOCTYPE html><meta charset=\"utf-8\"><p>"),
            "{doctype}"
        );
        assert!(to_unicode("<meta charset>").starts_with("<meta charset=\"utf-8\">"));
        assert!(to_unicode("<meta charset=>").starts_with("<meta charset=utf-8>"));
        assert!(to_unicode("<meta charset='iso-8859-1'>").starts_with("<meta charset='utf-8'>"));
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
        let html = "<p>Office: ಕನ್ನಡ &amp; ನಾಡು &rarr; ಕ&zwnj;ನ</p>\
                    <script>var s = 'ಕ';</script><title>ಕ</title>";
        let (out, report) = run(html, Direction::ToNudi);
        assert!(
            out.contains(
                "<p>Office: <span style=\"font-family:'Nudi 01 e'\">PÀ£ÀßqÀ &amp; £ÁqÀÄ</span> &rarr; "
            ),
            "{out}"
        );
        assert!(
            out.contains("<script>var s = 'ಕ';</script><title>ಕ</title>"),
            "{out}"
        );
        assert!(
            report.rows.iter().any(|r| r.kind == Kind::Unencodable),
            "{:?}",
            report.rows
        );
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
