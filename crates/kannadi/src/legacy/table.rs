//! The glyph table in `data/nudi.tsv`, parsed once and shared by both directions.

use crate::chars::VIRAMA;

use super::windows_1252;

/// What a glyph, or a glyph sequence, means.
#[derive(Debug)]
pub(super) enum Entry {
    Letter(String),
    Base(String),
    Sign(String),
    Virama,
    Ottu(char),
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
}

pub(super) struct Row {
    pub(super) key: String,
    pub(super) entry: Entry,
    pub(super) prefer: Prefer,
}

/// The rows of `data/nudi.tsv` in table order.
pub(super) fn rows() -> Vec<Row> {
    parse(include_str!("../../data/nudi.tsv"))
}

fn parse(source: &str) -> Vec<Row> {
    let mut rows = Vec::new();
    for line in source.lines() {
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let mut cols = line.split('\t');
        let codes = cols.next().unwrap_or_default();
        let kind = cols.next().unwrap_or_default();
        let value = cols.next().unwrap_or_default();
        let prefer = parse_prefer(cols.next().unwrap_or_default());
        let key: String = codes
            .split_whitespace()
            .map(|h| windows_1252(u8::from_str_radix(h, 16).expect("hex glyph code in nudi.tsv")))
            .collect();
        let first = value.chars().next();
        let entry = match kind {
            "letter" => Entry::Letter(value.to_string()),
            "base" => Entry::Base(value.to_string()),
            "sign" if first == Some(VIRAMA) => Entry::Virama,
            "sign" => Entry::Sign(value.to_string()),
            "ottu" => Entry::Ottu(first.expect("ottu needs a consonant")),
            "arka" => Entry::Arka,
            "mod" => Entry::Mod(first.expect("mod needs a sign")),
            "aspirate" => Entry::Aspirate,
            "ignore" => Entry::Ignore,
            "literal" => Entry::Literal(value.to_string()),
            other => panic!("unknown kind {other:?} in nudi.tsv"),
        };
        rows.push(Row { key, entry, prefer });
    }
    rows
}

fn parse_prefer(column: &str) -> Prefer {
    match column.split_once(' ') {
        None if column.is_empty() => Prefer::Otherwise,
        None if column == "after-ottu" => Prefer::AfterOttu,
        Some(("on", chars)) => Prefer::On(chars.replace(' ', "")),
        Some(("with", chars)) => Prefer::With(chars.replace(' ', "")),
        _ => panic!("unknown preference {column:?} in nudi.tsv"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preferences_parse() {
        assert_eq!(parse_prefer(""), Prefer::Otherwise);
        assert_eq!(parse_prefer("after-ottu"), Prefer::AfterOttu);
        assert_eq!(parse_prefer("on ಪ ಫ ವ"), Prefer::On("ಪಫವ".into()));
        assert_eq!(parse_prefer("with ಿ"), Prefer::With("ಿ".into()));
    }

    #[test]
    fn the_built_in_table_parses() {
        assert!(rows().len() > 150);
    }
}
