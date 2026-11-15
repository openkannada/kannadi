//! Converts 1 piece of text in either direction and lists what went wrong.

use std::ops::Range;

use kannadi::legacy::{nudi_to_unicode, unicode_to_nudi, Options};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    ToUnicode,
    ToNudi,
}

#[derive(Clone, Copy, Debug)]
pub struct Settings {
    pub direction: Direction,
    pub options: Options,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
}

/// A byte offset in the input text, or a byte range in the output text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Position {
    Input(usize),
    Output(Range<usize>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Issue {
    pub kind: Kind,
    pub at: Position,
    pub detail: String,
}

pub struct Converted {
    pub text: String,
    pub issues: Vec<Issue>,
}

pub fn convert(text: &str, settings: &Settings) -> Converted {
    match settings.direction {
        Direction::ToUnicode => {
            let c = nudi_to_unicode(text, &settings.options);
            let unknown = c.unknown.iter().map(|u| Issue {
                kind: Kind::UnknownGlyph,
                at: Position::Input(u.offset),
                detail: describe_glyph(u.glyph),
            });
            let malformed = c.malformed.iter().map(|r| Issue {
                kind: Kind::Malformed,
                at: Position::Output(r.clone()),
                detail: c.text[r.clone()].to_string(),
            });
            let issues = unknown.chain(malformed).collect();
            Converted {
                text: c.text,
                issues,
            }
        }
        Direction::ToNudi => {
            let e = unicode_to_nudi(text);
            let unencodable = e.unencodable.iter().map(|u| Issue {
                kind: Kind::Unencodable,
                at: Position::Input(u.offset),
                detail: describe_char(u.character),
            });
            let malformed = e.malformed.iter().map(|r| Issue {
                kind: Kind::Malformed,
                at: Position::Input(r.start),
                detail: text[r.clone()].to_string(),
            });
            let issues = unencodable.chain(malformed).collect();
            Converted {
                text: e.text,
                issues,
            }
        }
    }
}

pub fn describe_char(c: char) -> String {
    if c.is_control() || c.is_whitespace() {
        format!("U+{:04X}", u32::from(c))
    } else {
        format!("{c} (U+{:04X})", u32::from(c))
    }
}

fn describe_glyph(g: char) -> String {
    match kannadi::legacy::windows_1252_byte(g) {
        Some(b) => format!("byte {b:02X}"),
        None => describe_char(g),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(direction: Direction) -> Settings {
        Settings {
            direction,
            options: Options::default(),
        }
    }

    #[test]
    fn to_unicode_lists_unknown_and_malformed() {
        let c = convert("PÀ\u{F1} Á", &settings(Direction::ToUnicode));
        assert_eq!(c.text, "ಕ\u{F1} ಾ");
        assert_eq!(c.issues[0].kind, Kind::UnknownGlyph);
        assert_eq!(c.issues[0].detail, "byte F1");
        assert_eq!(c.issues[1].kind, Kind::Malformed);
        assert_eq!(c.issues[1].at, Position::Output(6..9));
    }

    #[test]
    fn to_nudi_lists_malformed_input() {
        let c = convert("ಕ ಕೇೂ", &settings(Direction::ToNudi));
        assert_eq!(c.issues[0].kind, Kind::Malformed);
        assert_eq!(c.issues[0].at, Position::Input(4));
        assert_eq!(c.issues[0].detail, "ಕೇೂ");
    }

    #[test]
    fn to_nudi_lists_unencodable() {
        let c = convert("ಕ ಌ", &settings(Direction::ToNudi));
        assert_eq!(c.issues.len(), 1);
        assert_eq!(c.issues[0].at, Position::Input(4));
        assert_eq!(c.issues[0].detail, "ಌ (U+0C8C)");
    }
}
