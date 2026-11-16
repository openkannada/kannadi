//! The benchmark corpus in `corpus/` converts to its expected text in both
//! directions. It is read from the repository at test time.

use kannadi::legacy::{decode_windows_1252, nudi_to_unicode, unicode_to_nudi, Options};

struct Line {
    id: String,
    status: String,
    legacy: String,
    unicode: String,
}

fn corpus() -> Vec<Line> {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../corpus/nudi-2002.tsv");
    let source = std::fs::read_to_string(path).expect("corpus/nudi-2002.tsv");
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
            Line {
                id: cols[0].to_string(),
                status: cols[1].to_string(),
                legacy: decode_windows_1252(&bytes),
                unicode: cols[4].to_string(),
            }
        })
        .collect()
}

#[test]
fn every_line_converts_to_its_expected_text() {
    let lines = corpus();
    assert!(lines.len() >= 200);
    for line in &lines {
        assert!(
            matches!(line.status.as_str(), "verified" | "pending" | "unchecked"),
            "{}",
            line.id
        );
        let converted = nudi_to_unicode(&line.legacy, &Options::default());
        assert_eq!(converted.text, line.unicode, "line {}", line.id);
    }
}

#[test]
fn most_lines_encode_back_as_typed() {
    let lines = corpus();
    let same = lines
        .iter()
        .filter(|l| unicode_to_nudi(&l.unicode).text == l.legacy)
        .count();
    assert!(same * 10 >= lines.len() * 9, "{same} of {}", lines.len());
}
