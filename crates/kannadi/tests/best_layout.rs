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
    assert_eq!(nudi, 0, "{nudi} of {nudi_words} Nudi words");
    assert!(
        akruti * 100 <= akruti_words,
        "{akruti} of {akruti_words} Akruti words"
    );
}

/// The corpus was not used to choose the rule.
#[test]
fn corpus_words_find_their_layout() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../corpus/nudi-2002.tsv");
    let corpus = std::fs::read_to_string(path).expect(path);
    let (nudi, words) = wrong(&corpus, 3, Layout::Nudi);
    eprintln!("corpus {nudi} of {words}");
    assert!(nudi * 200 <= words, "{nudi} of {words} corpus words");
}
