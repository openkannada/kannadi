//! `best_layout` on the words of the golden fixtures, whose layouts are known.

use kannadi::legacy::{best_layout, decode_windows_1252, Layout};

/// The words of the legacy column, the `column`th, of a table of hex codes.
fn words(table: &str, column: usize) -> Vec<String> {
    table
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .flat_map(|row| {
            let bytes: Vec<u8> = row
                .split('\t')
                .nth(column)
                .unwrap()
                .split_whitespace()
                .map(|h| u8::from_str_radix(h, 16).unwrap())
                .collect();
            decode_windows_1252(&bytes)
                .split(' ')
                .filter(|w| !w.is_ascii())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

fn wrong(table: &str, column: usize, layout: Layout) -> (usize, usize) {
    let words = words(table, column);
    let wrong = words.iter().filter(|w| best_layout(w) != layout).count();
    (wrong, words.len())
}

#[test]
fn single_words_mostly_find_their_layout() {
    let (nudi, nudi_words) = wrong(include_str!("fixtures/nudi.tsv"), 0, Layout::Nudi);
    let (akruti, akruti_words) = wrong(include_str!("fixtures/akruti.tsv"), 0, Layout::Akruti);
    let (shree, shree_words) = wrong(
        include_str!("fixtures/shree-lipi.tsv"),
        0,
        Layout::ShreeLipi,
    );
    assert_eq!(nudi, 0, "{nudi} of {nudi_words} Nudi words");
    assert!(
        akruti * 100 <= akruti_words,
        "{akruti} of {akruti_words} Akruti words"
    );
    assert!(
        shree * 50 <= shree_words,
        "{shree} of {shree_words} Shree-Lipi words"
    );
}

/// Whole lines carry enough text to tell the layouts apart, the Assembly layout's
/// included, whose single words often read cleanly in Nudi too.
#[test]
fn whole_lines_find_their_layout() {
    for (table, layout) in [
        (include_str!("fixtures/nudi.tsv"), Layout::Nudi),
        (include_str!("fixtures/akruti.tsv"), Layout::Akruti),
        (include_str!("fixtures/shree-lipi.tsv"), Layout::ShreeLipi),
        (include_str!("fixtures/nudi-b.tsv"), Layout::NudiB),
    ] {
        for row in table
            .lines()
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
        {
            let bytes: Vec<u8> = row
                .split('\t')
                .next()
                .unwrap()
                .split_whitespace()
                .map(|h| u8::from_str_radix(h, 16).unwrap())
                .collect();
            let line = decode_windows_1252(&bytes);
            assert_eq!(best_layout(&line), layout, "{line}");
        }
    }
}

/// The corpus was not used to choose the rule.
#[test]
fn corpus_words_find_their_layout() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../corpus/nudi-2002.tsv");
    let corpus = std::fs::read_to_string(path).expect(path);
    let (nudi, words) = wrong(&corpus, 3, Layout::Nudi);
    assert!(nudi * 200 <= words, "{nudi} of {words} corpus words");
}
