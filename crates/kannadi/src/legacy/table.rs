//! The glyph tables in `data/`, 1 per layout, shared by both directions.
//!
//! Each line of a table is a glyph sequence and what it means, in tab-separated
//! columns: the glyph codes (hex, as Windows-1252 bytes), the kind, the Kannada
//! value, and optionally a preference. Lines starting with `#` are comments.
//!
//! Kinds:
//!
//! - `letter`: starts a syllable; the value is a letter, optionally with a vowel
//!   sign, or a whole cluster such as ಶ್ರೀ.
//! - `base`: starts a syllable whose consonant shape needs a following sign (the
//!   head stroke for the inherent a, a vowel sign, or the virama).
//! - `sign`: a vowel sign or part of one, added to the current syllable. An empty
//!   value is the head stroke; the virama is a sign.
//! - `ottu`: subscript consonants added to the current syllable's cluster. The
//!   value is 1 consonant, or several drawn as 1 glyph, optionally followed by
//!   the vowel sign drawn with them.
//! - `arka`: the arkavattu, ರ್ written before the cluster.
//! - `mod`: anusvara or visarga.
//! - `aspirate`: turns ಪ ಬ ಡ ದ into ಫ ಭ ಢ ಧ.
//! - `ignore`: a spacing glyph with no text.
//! - `literal`: punctuation, digits and spaces drawn as themselves.
//!
//! Preferences, for Unicode to legacy text; otherwise the first glyph with a
//! value is written:
//!
//! - `on X`: the syllable's consonant is one of X.
//! - `with X`: the syllable's vowel sign contains one of X.
//! - `after-ottu`: the glyph comes right after a subscript consonant.
//! - `read`: a way of typing that is read but never written, such as a sign
//!   typed after a space.

use std::sync::OnceLock;

use crate::chars::{class, Class, VIRAMA};

use super::{windows_1252_char, Layout};

/// What a glyph, or a glyph sequence, means.
#[derive(Debug)]
pub(super) enum Entry {
    Letter(String),
    Base(String),
    Sign(String),
    Virama,
    /// Subscript consonants drawn as 1 glyph, and the vowel sign drawn with them.
    Ottu(Vec<char>, Option<char>),
    Arka,
    Mod(char),
    Aspirate,
    Ignore,
    Literal(String),
}

/// When the encoder prefers a glyph over others with the same meaning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Prefer {
    /// The default when no other preference holds; the first such glyph wins.
    Otherwise,
    /// The syllable's consonant is one of these.
    On(String),
    /// The syllable's vowel sign contains one of these.
    With(String),
    /// The glyph written just before it is a subscript consonant.
    AfterOttu,
    /// Never written.
    Read,
}

pub(super) struct Row {
    pub(super) key: String,
    pub(super) entry: Entry,
    pub(super) prefer: Prefer,
}

/// The file name of a layout's table, for messages about it.
pub(super) fn file_name(layout: Layout) -> &'static str {
    match layout {
        Layout::Nudi => "nudi.tsv",
        Layout::Akruti => "akruti.tsv",
        Layout::ShreeLipi => "shree-lipi.tsv",
        Layout::NudiB => "nudi-b.tsv",
        Layout::KanEng => "kan-eng.tsv",
    }
}

fn contents(layout: Layout) -> &'static str {
    match layout {
        Layout::Nudi => include_str!("../../data/nudi.tsv"),
        Layout::Akruti => include_str!("../../data/akruti.tsv"),
        Layout::ShreeLipi => include_str!("../../data/shree-lipi.tsv"),
        Layout::NudiB => include_str!("../../data/nudi-b.tsv"),
        Layout::KanEng => include_str!("../../data/kan-eng.tsv"),
    }
}

/// Whether the layout's table keeps the ASCII letters as Latin letters.
pub(super) fn draws_latin(layout: Layout) -> bool {
    static LATIN: OnceLock<Vec<bool>> = OnceLock::new();
    let latin = LATIN.get_or_init(|| {
        Layout::ALL
            .iter()
            .map(|&l| {
                rows(l).iter().any(|row| {
                    matches!(&row.entry, Entry::Literal(v) if v.chars().all(|c| c.is_ascii_alphabetic()))
                })
            })
            .collect()
    });
    latin[layout as usize]
}

/// The rows of a layout's table in table order.
pub(super) fn rows(layout: Layout) -> Vec<Row> {
    parse(contents(layout), file_name(layout))
}

fn parse(source: &str, file: &str) -> Vec<Row> {
    let mut rows = Vec::new();
    for line in source.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let mut cols = line.split('\t');
        let codes = cols.next().unwrap_or_default();
        let kind = cols.next().unwrap_or_default();
        let value = cols.next().unwrap_or_default();
        let prefer = parse_prefer(cols.next().unwrap_or_default(), file);
        let key: String = codes
            .split_whitespace()
            .map(|h| {
                let code = u8::from_str_radix(h, 16);
                windows_1252_char(
                    code.unwrap_or_else(|_| panic!("{file}: {h:?} is not a hex code")),
                )
            })
            .collect();
        let first = value.chars().next();
        let entry = match kind {
            "letter" => Entry::Letter(value.to_string()),
            "base" => Entry::Base(value.to_string()),
            "sign" if first == Some(VIRAMA) => Entry::Virama,
            "sign" => Entry::Sign(value.to_string()),
            "ottu" => parse_ottu(value, file),
            "arka" => Entry::Arka,
            "mod" => Entry::Mod(first.unwrap_or_else(|| panic!("{file}: mod needs a sign"))),
            "aspirate" => Entry::Aspirate,
            "ignore" => Entry::Ignore,
            "literal" => Entry::Literal(value.to_string()),
            other => panic!("{file}: unknown kind {other:?}"),
        };
        let after_space = key.chars().count() > 1 && key.starts_with(char::is_whitespace);
        assert!(
            !after_space || prefer == Prefer::Read,
            "{file}: {codes} starts with a space, so it must be marked read"
        );
        rows.push(Row { key, entry, prefer });
    }
    rows
}

/// 1 or more consonants, then at most 1 vowel sign.
fn parse_ottu(value: &str, file: &str) -> Entry {
    let consonants: Vec<char> = value
        .chars()
        .take_while(|&c| class(c) == Class::Consonant)
        .collect();
    let mut rest = value.chars().skip(consonants.len());
    let sign = rest.next();
    let valid = !consonants.is_empty()
        && sign.map_or(true, |s| class(s) == Class::VowelSign)
        && rest.next().is_none();
    assert!(
        valid,
        "{file}: ottu {value:?} is not consonants and a vowel sign"
    );
    Entry::Ottu(consonants, sign)
}

fn parse_prefer(column: &str, file: &str) -> Prefer {
    match column.split_once(' ') {
        None if column.is_empty() => Prefer::Otherwise,
        None if column == "after-ottu" => Prefer::AfterOttu,
        None if column == "read" => Prefer::Read,
        Some(("on", chars)) => Prefer::On(chars.replace(' ', "")),
        Some(("with", chars)) => Prefer::With(chars.replace(' ', "")),
        _ => panic!("{file}: unknown preference {column:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_parse() {
        assert_eq!(parse_prefer("", "t"), Prefer::Otherwise);
        assert_eq!(parse_prefer("after-ottu", "t"), Prefer::AfterOttu);
        assert_eq!(parse_prefer("on ಪ ಫ ವ", "t"), Prefer::On("ಪಫವ".into()));
        assert_eq!(parse_prefer("with ಿ", "t"), Prefer::With("ಿ".into()));
        assert_eq!(parse_prefer("read", "t"), Prefer::Read);
    }

    #[test]
    fn rows_typed_after_a_space_must_be_read_only() {
        assert_eq!(parse("20 BD\tsign\tೃ\tread", "t").len(), 1);
        assert!(std::panic::catch_unwind(|| parse("20 BD\tsign\tೃ", "t")).is_err());
    }

    #[test]
    fn ottu_values_are_consonants_and_a_sign() {
        assert!(matches!(parse_ottu("ಕ", "t"), Entry::Ottu(c, None) if c == ['ಕ']));
        assert!(matches!(parse_ottu("ಟರ", "t"), Entry::Ottu(c, None) if c == ['ಟ', 'ರ']));
        assert!(matches!(parse_ottu("ಕೃ", "t"), Entry::Ottu(c, Some('ೃ')) if c == ['ಕ']));
        for bad in ["", "ೃ", "ಕ್ಷ", "ಕೃೃ"] {
            assert!(
                std::panic::catch_unwind(|| parse_ottu(bad, "t")).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn the_built_in_tables_parse() {
        for layout in Layout::ALL {
            assert!(rows(layout).len() > 150, "{layout}");
        }
    }
}
