//! The glyph tables in `data/`, 1 per layout, shared by both directions.

use crate::chars::VIRAMA;

use super::{windows_1252, Layout};

/// What a glyph, or a glyph sequence, means.
#[derive(Debug)]
pub(super) enum Entry {
    Letter(String),
    Base(String),
    Sign(String),
    Virama,
    Ottu(char),
    /// Several subscript consonants drawn as 1 glyph, optionally followed by a
    /// vowel sign drawn with them. Read only; the encoder writes each subscript.
    Subscripts(String),
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

/// The file name and contents of a layout's table.
pub(super) fn source(layout: Layout) -> (&'static str, &'static str) {
    match layout {
        Layout::Nudi => ("nudi.tsv", include_str!("../../data/nudi.tsv")),
        Layout::Akruti => ("akruti.tsv", include_str!("../../data/akruti.tsv")),
    }
}

/// The rows of a layout's table in table order.
pub(super) fn rows(layout: Layout) -> Vec<Row> {
    let (file, text) = source(layout);
    parse(text, file)
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
                windows_1252(code.unwrap_or_else(|_| panic!("{file}: {h:?} is not a hex code")))
            })
            .collect();
        let first = value.chars().next();
        let entry = match kind {
            "letter" => Entry::Letter(value.to_string()),
            "base" => Entry::Base(value.to_string()),
            "sign" if first == Some(VIRAMA) => Entry::Virama,
            "sign" => Entry::Sign(value.to_string()),
            "ottu" if value.chars().count() > 1 => Entry::Subscripts(value.to_string()),
            "ottu" => {
                Entry::Ottu(first.unwrap_or_else(|| panic!("{file}: ottu needs a consonant")))
            }
            "arka" => Entry::Arka,
            "mod" => Entry::Mod(first.unwrap_or_else(|| panic!("{file}: mod needs a sign"))),
            "aspirate" => Entry::Aspirate,
            "ignore" => Entry::Ignore,
            "literal" => Entry::Literal(value.to_string()),
            other => panic!("{file}: unknown kind {other:?}"),
        };
        rows.push(Row { key, entry, prefer });
    }
    rows
}

fn parse_prefer(column: &str, file: &str) -> Prefer {
    match column.split_once(' ') {
        None if column.is_empty() => Prefer::Otherwise,
        None if column == "after-ottu" => Prefer::AfterOttu,
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
    }

    #[test]
    fn the_built_in_tables_parse() {
        for layout in Layout::ALL {
            assert!(rows(layout).len() > 150, "{layout}");
        }
    }
}
