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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    UnknownGlyph,
    Malformed,
    Unencodable,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::UnknownGlyph => "unknown glyph",
            Kind::Malformed => "malformed syllable",
            Kind::Unencodable => "unencodable",
        }
    }

    pub fn plural(self) -> &'static str {
        match self {
            Kind::UnknownGlyph => "unknown glyphs",
            Kind::Malformed => "malformed syllables",
            Kind::Unencodable => "unencodable characters",
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
        Settings { direction }
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
