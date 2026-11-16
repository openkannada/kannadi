//! The benchmark corpus in `corpus/` converts to its expected text in both
//! directions. It is read from the repository at test time.

use std::collections::HashSet;

use kannadi::legacy::{decode_windows_1252, nudi_to_unicode, unicode_to_nudi, Options};

struct Line {
    id: String,
    status: String,
    legacy: String,
    unicode: String,
}

fn corpus() -> Vec<Line> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../corpus/nudi-2002.tsv");
    let source = std::fs::read_to_string(path).expect(path);
    source
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|row| {
            let cols: Vec<&str> = row.split('\t').collect();
            assert_eq!(cols.len(), 6, "{row}");
            let bytes: Vec<u8> = cols[3]
                .split_whitespace()
                .map(|h| u8::from_str_radix(h, 16).expect("hex"))
                .collect();
            let line = Line {
                id: cols[0].to_string(),
                status: cols[1].to_string(),
                legacy: decode_windows_1252(&bytes),
                unicode: cols[4].to_string(),
            };
            let status = line.status.as_str();
            assert!(
                matches!(status, "verified" | "pending" | "unchecked"),
                "{}",
                line.id
            );
            line
        })
        .collect()
}

#[test]
fn every_line_converts_to_its_expected_text() {
    let lines = corpus();
    assert_eq!(lines.len(), 207);
    let ids: HashSet<&str> = lines.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(ids.len(), lines.len(), "ids must be unique");
    for line in &lines {
        let converted = nudi_to_unicode(&line.legacy, &Options::default());
        assert_eq!(converted.text, line.unicode, "line {}", line.id);
    }
}

/// Lines 103 and 166 keep typing habits the encoder does not copy: a dead
/// consonant typed as its own syllable, and an extra spacing glyph.
#[test]
fn lines_encode_back_as_typed() {
    for line in corpus() {
        let typed = unicode_to_nudi(&line.unicode).text == line.legacy;
        let habit = matches!(line.id.as_str(), "103" | "166");
        assert_eq!(typed, !habit, "line {} ({})", line.id, line.status);
    }
}
