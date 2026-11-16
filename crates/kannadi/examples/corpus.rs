//! Measures the legacy converter on the benchmark corpus:
//! `cargo run --example corpus [corpus/nudi-2002.tsv]`.
//!
//! For each status (verified, pending, unchecked) it prints the lines that convert
//! to their expected text exactly, the akshara error rate (edit distance over
//! aksharas, divided by the expected aksharas), lines with an unknown glyph or a
//! malformed syllable, and lines that encode back to the bytes that were typed.

use std::collections::BTreeMap;

use kannadi::legacy::{decode_windows_1252, nudi_to_unicode, unicode_to_nudi, Options};

#[derive(Default)]
struct Tally {
    lines: usize,
    exact: usize,
    errors: usize,
    aksharas: usize,
    flagged: usize,
    encoded_as_typed: usize,
}

fn main() -> std::io::Result<()> {
    let default = concat!(env!("CARGO_MANIFEST_DIR"), "/../../corpus/nudi-2002.tsv");
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| default.to_string());
    let source = std::fs::read_to_string(&path)
        .map_err(|e| std::io::Error::new(e.kind(), format!("{path}: {e}")))?;
    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    for row in source
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
    {
        let cols: Vec<&str> = row.split('\t').collect();
        let [id, status, _, hex, expected, ..] = cols[..] else {
            eprintln!("skipping a row with {} columns", cols.len());
            continue;
        };
        let Ok(bytes) = hex
            .split_whitespace()
            .map(|h| u8::from_str_radix(h, 16))
            .collect::<Result<Vec<u8>, _>>()
        else {
            eprintln!("skipping line {id}: its glyph codes are not hex");
            continue;
        };
        let legacy = decode_windows_1252(&bytes);
        let converted = nudi_to_unicode(&legacy, &Options::default());
        let expected_aksharas = kannadi::split(expected);
        let t = tallies.entry(status.to_string()).or_default();
        t.lines += 1;
        t.exact += usize::from(converted.text == expected);
        t.errors += distance(&kannadi::split(&converted.text), &expected_aksharas);
        t.aksharas += expected_aksharas.len();
        t.flagged += usize::from(!converted.is_clean());
        t.encoded_as_typed += usize::from(unicode_to_nudi(expected).text == legacy);
    }
    println!("status     lines  exact  akshara error rate  flagged  encoded as typed");
    println!("{}", "-".repeat(73));
    for (status, t) in &tallies {
        let rate = 100.0 * t.errors as f64 / t.aksharas.max(1) as f64;
        println!(
            "{status:<10} {:>5}  {:>5}  {rate:>17.2}%  {:>7}  {:>16}",
            t.lines, t.exact, t.flagged, t.encoded_as_typed
        );
    }
    if !tallies.contains_key("verified") {
        println!("No line is verified yet, so these figures compare the converter with itself.");
    }
    Ok(())
}

/// Levenshtein distance between 2 sequences.
fn distance(a: &[&str], b: &[&str]) -> usize {
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, x) in a.iter().enumerate() {
        let mut current = vec![i + 1];
        for (j, y) in b.iter().enumerate() {
            let substitute = previous[j] + usize::from(x != y);
            current.push(substitute.min(previous[j + 1] + 1).min(current[j] + 1));
        }
        previous = current;
    }
    previous[b.len()]
}
