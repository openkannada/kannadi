//! Converts 1 piece of text in either direction and lists what went wrong.

use kannadi::legacy::{nudi_to_unicode, unicode_to_nudi, windows_1252_byte, Options};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    ToUnicode(Options),
    ToNudi,
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub direction: Direction,
    pub fonts: Fonts,
}

impl Settings {
    pub fn new(direction: Direction) -> Self {
        Settings {
            direction,
            fonts: Fonts::default(),
        }
    }
}

/// Font names for documents that record fonts, such as HTML.
#[derive(Clone, Debug)]
pub struct Fonts {
    /// Set on text converted to Unicode.
    pub unicode: String,
    /// Set on text converted to Nudi.
    pub legacy: String,
    /// More font names to treat as Nudi and Baraha ANSI fonts.
    pub extra_legacy: Vec<String>,
}

impl Default for Fonts {
    fn default() -> Self {
        Fonts {
            unicode: "Noto Sans Kannada".to_string(),
            legacy: "Nudi 01 e".to_string(),
            extra_legacy: Vec::new(),
        }
    }
}

impl Fonts {
    pub fn is_legacy(&self, font: &str) -> bool {
        crate::fonts::is_legacy(font, &self.extra_legacy)
    }

    /// A `font-family` list with each legacy family replaced by the Unicode font,
    /// in `quote` marks when given. A trailing `!important` is kept.
    pub fn unicode_family(&self, list: &str, quote: Option<char>) -> String {
        let trimmed = list.trim_end();
        let (families, important) = match trimmed.strip_suffix("!important") {
            Some(rest) => (rest.trim_end(), &list[rest.trim_end().len()..]),
            None => (list, ""),
        };
        let replaced =
            crate::fonts::replace_legacy(families, &self.unicode, quote, &self.extra_legacy);
        format!("{replaced}{important}")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    UnknownGlyph,
    Malformed,
    Unencodable,
    /// Text left out because it cannot be read, such as a PDF font whose glyph
    /// codes were renumbered.
    NotConverted,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::UnknownGlyph => "unknown glyph",
            Kind::Malformed => "malformed syllable",
            Kind::Unencodable => "unencodable",
            Kind::NotConverted => "part not converted",
        }
    }

    pub fn plural(self) -> &'static str {
        match self {
            Kind::UnknownGlyph => "unknown glyphs",
            Kind::Malformed => "malformed syllables",
            Kind::Unencodable => "unencodable characters",
            Kind::NotConverted => "parts not converted",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub kind: Kind,
    /// 1-based line in the converted piece of text.
    pub line: usize,
    pub detail: String,
    /// Unicode text around the issue.
    pub context: String,
}

pub struct Converted {
    pub text: String,
    /// In order of position.
    pub issues: Vec<Issue>,
}

pub fn convert(text: &str, settings: &Settings) -> Converted {
    let input = Lines::new(text);
    match settings.direction {
        Direction::ToUnicode(options) => {
            let c = nudi_to_unicode(text, &options);
            let output = Lines::new(&c.text);
            let unknown = c.unknown.iter().map(|u| {
                let around = nudi_to_unicode(window(text, &input, u.offset), &options).text;
                (
                    u.offset,
                    Issue {
                        kind: Kind::UnknownGlyph,
                        line: input.line(u.offset),
                        detail: describe_glyph(u.glyph),
                        context: around,
                    },
                )
            });
            // Malformed ranges are in the output; their input position is unknown,
            // so they sort by line after the unknown glyphs of that line.
            let malformed = c.malformed.iter().map(|r| {
                let line = output.line(r.start);
                (
                    input.start(line),
                    Issue {
                        kind: Kind::Malformed,
                        line,
                        detail: c.text[r.clone()].to_string(),
                        context: window(&c.text, &output, r.start).to_string(),
                    },
                )
            });
            Converted {
                issues: in_order(unknown.chain(malformed)),
                text: c.text,
            }
        }
        Direction::ToNudi => {
            let e = unicode_to_nudi(text);
            let issue = |kind, at: usize, detail| {
                let context = window(text, &input, at).to_string();
                let line = input.line(at);
                (
                    at,
                    Issue {
                        kind,
                        line,
                        detail,
                        context,
                    },
                )
            };
            let unencodable = e
                .unencodable
                .iter()
                .map(|u| issue(Kind::Unencodable, u.offset, describe_char(u.character)));
            let malformed = e
                .malformed
                .iter()
                .map(|r| issue(Kind::Malformed, r.start, text[r.clone()].to_string()));
            Converted {
                issues: in_order(unencodable.chain(malformed)),
                text: e.text,
            }
        }
    }
}

fn in_order(issues: impl Iterator<Item = (usize, Issue)>) -> Vec<Issue> {
    let mut issues: Vec<(usize, Issue)> = issues.collect();
    issues.sort_by_key(|(at, issue)| (issue.line, *at));
    issues.into_iter().map(|(_, issue)| issue).collect()
}

/// Line numbers by binary search over the line starts.
struct Lines {
    starts: Vec<usize>,
}

impl Lines {
    fn new(text: &str) -> Self {
        let breaks = text.match_indices('\n').map(|(i, _)| i + 1);
        Lines {
            starts: std::iter::once(0).chain(breaks).collect(),
        }
    }

    /// The 1-based line holding byte `at`.
    fn line(&self, at: usize) -> usize {
        self.starts.partition_point(|&s| s <= at)
    }

    /// The byte where 1-based `line` starts.
    fn start(&self, line: usize) -> usize {
        self.starts
            .get(line.saturating_sub(1))
            .copied()
            .unwrap_or(0)
    }
}

/// Up to 30 characters on each side of byte `at`, within its line.
fn window<'t>(text: &'t str, lines: &Lines, at: usize) -> &'t str {
    let line = lines.line(at);
    let line_start = lines.start(line);
    let line_end = lines.starts.get(line).map_or(text.len(), |&next| next - 1);
    let start = text[line_start..at]
        .char_indices()
        .rev()
        .nth(29)
        .map_or(line_start, |(i, _)| line_start + i);
    let end = text[at..line_end]
        .char_indices()
        .nth(30)
        .map_or(line_end, |(i, _)| at + i);
    text[start..end].trim_end_matches('\r')
}

/// Byte ranges of Kannada text in `text`. Each starts and ends with a Kannada
/// character, and may hold spaces, joiners, digits and punctuation between.
pub fn kannada_runs(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut runs = Vec::new();
    let mut open: Option<std::ops::Range<usize>> = None;
    for (i, c) in text.char_indices() {
        if is_kannada_text(c) {
            let end = i + c.len_utf8();
            open = Some(open.map_or(i..end, |r| r.start..end));
        } else if !(c.is_whitespace()
            || c.is_ascii_punctuation()
            || c.is_ascii_digit()
            || c == '\u{200C}'
            || c == '\u{200D}')
        {
            runs.extend(open.take());
        }
    }
    runs.extend(open);
    runs
}

/// Kannada letters and marks, and the dandas (U+0964, U+0965) Kannada uses.
fn is_kannada_text(c: char) -> bool {
    kannadi::chars::is_kannada(c) || c == '\u{0964}' || c == '\u{0965}'
}

pub fn describe_char(c: char) -> String {
    if c.is_control() || c.is_whitespace() {
        format!("U+{:04X}", u32::from(c))
    } else {
        format!("{c} (U+{:04X})", u32::from(c))
    }
}

/// The glyph's code in the font, which is its Windows-1252 byte.
fn describe_glyph(g: char) -> String {
    match windows_1252_byte(g) {
        Some(b) => format!("glyph {b:02X}"),
        None => describe_char(g),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(direction: Direction) -> Settings {
        Settings::new(direction)
    }

    #[test]
    fn kannada_runs_stop_at_other_scripts() {
        let text = "Dept: ಕನ್ನಡ ನಾಡು, 2026। English ಕ";
        let runs: Vec<&str> = kannada_runs(text).into_iter().map(|r| &text[r]).collect();
        assert_eq!(runs, ["ಕನ್ನಡ ನಾಡು, 2026।", "ಕ"]);
    }

    fn to_unicode() -> Settings {
        settings(Direction::ToUnicode(Options::default()))
    }

    #[test]
    fn to_unicode_lists_issues_in_line_order() {
        let c = convert("PÀ Á\nPÀ\u{F1}\r\nÁ", &to_unicode());
        assert_eq!(c.text, "ಕ ಾ\nಕ\u{F1}\r\nಾ");
        let found: Vec<(usize, Kind)> = c.issues.iter().map(|i| (i.line, i.kind)).collect();
        assert_eq!(
            found,
            [
                (1, Kind::Malformed),
                (2, Kind::UnknownGlyph),
                (3, Kind::Malformed)
            ]
        );
        assert_eq!(c.issues[1].detail, "glyph F1");
        assert_eq!(c.issues[1].context, "ಕ\u{F1}");
    }

    #[test]
    fn to_nudi_lists_malformed_input() {
        let c = convert("ಕ ಕೇೂ", &settings(Direction::ToNudi));
        assert_eq!(c.issues[0].kind, Kind::Malformed);
        assert_eq!(c.issues[0].detail, "ಕೇೂ");
        assert_eq!(c.issues[0].context, "ಕ ಕೇೂ");
    }

    #[test]
    fn to_nudi_lists_unencodable() {
        let c = convert("ಕ ಌ", &settings(Direction::ToNudi));
        assert_eq!(c.issues.len(), 1);
        assert_eq!(c.issues[0].detail, "ಌ (U+0C8C)");
    }

    #[test]
    fn windows_stay_short_and_on_1_line() {
        let long = format!(
            "{}\n{}X{}\nz",
            "a".repeat(50),
            "b".repeat(50),
            "c".repeat(50)
        );
        let at = long.find('X').unwrap();
        let lines = Lines::new(&long);
        let w = window(&long, &lines, at);
        assert_eq!(w, format!("{}X{}", "b".repeat(30), "c".repeat(29)));
        assert_eq!(lines.line(0), 1);
        assert_eq!(lines.line(at), 2);
        assert_eq!(lines.line(long.len() - 1), 3);
    }
}
