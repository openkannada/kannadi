//! Property tests: the analyzer never splits an akshara, never panics, and gives keys
//! that analyze to themselves.

use kannadi_search::fold::fold;
use kannadi_search::Analyzer;
use proptest::prelude::*;

/// Kannada letters, signs, joiners, digits, a few Latin letters and spaces.
fn text() -> impl Strategy<Value = String> {
    let chars = prop_oneof![
        8 => (0x0C80u32..=0x0CF3).prop_map(|c| char::from_u32(c).unwrap_or('ಕ')),
        2 => Just(' '),
        1 => prop::sample::select(vec!['\u{200C}', '\u{200D}', '\u{00AD}', 'a', 'o', 'Z', '7', '\n', '.', 'À', 'Ä']),
    ];
    prop::collection::vec(chars, 0..40).prop_map(|v| v.into_iter().collect())
}

/// Words of well-formed aksharas separated by spaces.
fn kannada_words() -> impl Strategy<Value = String> {
    let consonant = (0x0C95u32..=0x0CB9).prop_map(|c| char::from_u32(c).unwrap_or('ಕ'));
    let sign = prop::sample::select(vec!["", "ಾ", "ಿ", "ು", "ೆ", "ೋ", "್", "ಂ", "\u{0CCD}\u{200C}"]);
    let akshara = (consonant, sign).prop_map(|(c, s)| format!("{c}{s}"));
    let word = prop::collection::vec(akshara, 1..6).prop_map(|a| a.concat());
    prop::collection::vec(word, 1..6).prop_map(|w| w.join(" "))
}

proptest! {
    #[test]
    fn offsets_are_inside_the_text_and_in_order(text in text()) {
        let tokens = Analyzer::new().analyze(&text);
        let mut last = 0;
        for (i, t) in tokens.iter().enumerate() {
            prop_assert_eq!(t.position, i);
            prop_assert!(t.offsets.start < t.offsets.end && t.offsets.end <= text.len());
            prop_assert!(text.is_char_boundary(t.offsets.start) && text.is_char_boundary(t.offsets.end));
            prop_assert!(t.offsets.start >= last || t.offsets.start == tokens[i - 1].offsets.start);
            last = t.offsets.start;
            prop_assert!(!t.text.is_empty());
        }
    }

    #[test]
    fn tokens_start_and_end_at_akshara_boundaries(text in kannada_words()) {
        let mut boundaries = vec![0];
        let mut at = 0;
        for a in kannadi::split(&text) {
            at += a.len();
            boundaries.push(at);
        }
        for t in Analyzer::new().analyze(&text) {
            prop_assert!(boundaries.contains(&t.offsets.start), "{:?} in {:?}", t, text);
            prop_assert!(boundaries.contains(&t.offsets.end), "{:?} in {:?}", t, text);
        }
    }

    #[test]
    fn folding_twice_is_folding_once(text in text()) {
        let once = fold(&text);
        prop_assert_eq!(fold(&once), once);
    }

    #[test]
    fn keys_are_their_own_keys(text in kannada_words()) {
        let analyzer = Analyzer::new();
        for key in analyzer.keys(&text) {
            prop_assert_eq!(analyzer.keys(&key), vec![key.clone()], "from {:?}", text);
        }
    }
}
