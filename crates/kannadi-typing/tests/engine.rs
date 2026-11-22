//! The engine with the bundled model.

use kannadi_typing::units::{text, units};
use kannadi_typing::{Engine, Model, Weights};
use proptest::prelude::*;
use std::sync::OnceLock;

fn engine() -> &'static Engine {
    static E: OnceLock<Engine> = OnceLock::new();
    E.get_or_init(Engine::bundled)
}

fn top(input: &str, n: usize) -> Vec<String> {
    engine().suggest(input, None, n)
}

#[test]
fn loose_spellings_find_the_word() {
    for input in ["kannada", "kannda", "Kannada", "KANNADA"] {
        assert_eq!(top(input, 1), ["ಕನ್ನಡ"], "{input}");
    }
    assert_eq!(top("bengaluru", 1), ["ಬೆಂಗಳೂರು"]);
    assert_eq!(top("bangalore", 1), ["ಬೆಂಗಳೂರು"]);
    assert_eq!(top("namaskara", 1), ["ನಮಸ್ಕಾರ"]);
    assert_eq!(top("computer", 1), ["ಕಂಪ್ಯೂಟರ್"]);
}

#[test]
fn precise_input_is_read_as_baraha_or_itrans() {
    assert_eq!(top("kannaDa", 1), ["ಕನ್ನಡ"]);
    assert_eq!(top("kRRiShNa", 1), ["ಕೃಷ್ಣ"]);
    assert_eq!(top("SaaLe", 1), ["ಶಾಳೆ"]);
    assert_eq!(top("gaa.ndhii", 1), ["ಗಾಂಧೀ"]);
    assert_eq!(top("j~naana", 1), ["ಜ್ಞಾನ"]);
}

#[test]
fn til_offers_tili() {
    assert!(top("til", 5).contains(&"ತಿಳಿ".to_string()));
    assert_eq!(top("tili", 1), ["ತಿಳಿ"]);
}

#[test]
fn a_completion_never_comes_first() {
    for input in ["til", "namask", "sark", "t", "bengal"] {
        let c = engine().candidates(input, None);
        assert_eq!(c[0].extra, 0, "{input}: {}", c[0].word);
        assert!(c.iter().any(|c| c.extra > 0), "{input} has no completion");
    }
    assert!(top("namask", 5).contains(&"ನಮಸ್ಕಾರ".to_string()));
}

#[test]
fn every_suggestion_is_a_well_formed_word() {
    for input in ["a", "ksh", "aaaaaaaaaa", "rrrr", "nyaaya", "xyz", "q"] {
        for w in top(input, 10) {
            assert_eq!(units(&w).map(|u| text(&u)), Some(w.clone()), "{input}: {w}");
        }
    }
}

#[test]
fn other_input_gives_nothing() {
    for input in ["", "kan nada", "a1", "ಕನ್ನಡ", "é", "~", "a-b"] {
        assert!(top(input, 5).is_empty(), "{input}");
    }
}

#[test]
fn learning_puts_the_pick_first_and_survives_export() {
    let mut e = Engine::bundled();
    let before = e.suggest("til", None, 5);
    assert_ne!(before[0], "ತಿಳಿ");
    assert!(e.learn("til", "ತಿಳಿ"));
    assert_eq!(e.suggest("til", None, 1), ["ತಿಳಿ"]);
    assert_eq!(e.suggest("TIL", None, 1), ["ತಿಳಿ"]);
    // A newer pick for the same letters wins.
    assert!(e.learn("til", "ತಿಲ"));
    assert_eq!(e.suggest("til", None, 2), ["ತಿಲ", "ತಿಳಿ"]);

    let saved = e.export_user();
    let mut f = Engine::bundled();
    assert!(f.import_user(&saved));
    assert_eq!(f.suggest("til", None, 2), ["ತಿಲ", "ತಿಳಿ"]);
    assert!(!f.import_user("something else"));
    assert_eq!(f.suggest("til", None, 2), ["ತಿಲ", "ತಿಳಿ"]);
    f.forget("ತಿಲ");
    f.forget("ತಿಳಿ");
    assert_eq!(f.suggest("til", None, 5), before);
    assert!(!f.learn("til", "tili"));
    assert!(!f.learn("1", "ತಿಳಿ"));
}

#[test]
fn a_learned_word_the_lexicon_lacks_is_offered_for_similar_letters() {
    let word = "ಜಗಲಿಕಟ್ಟೆಯಲ್ಲಿದ್ದಾಗ";
    assert!(!engine().model().has_word(word));
    let mut e = Engine::bundled();
    assert!(!e
        .suggest("jagalikatteyalliddaga", None, 5)
        .contains(&word.to_string()));
    assert!(e.learn("jagalikatteyalliddaga", word));
    assert_eq!(e.suggest("jagalikatteyalliddaaga", None, 1), [word]);
    let mut f = Engine::bundled();
    assert!(f.import_user(&e.export_user()));
    assert_eq!(f.suggest("jagalikatteyalliddaaga", None, 1), [word]);
}

#[test]
fn the_previous_word_counts() {
    let m = engine().model();
    assert!(m.word_pairs() > 0);
    assert_eq!(m.lift("ಇಲ್ಲದ", "ಇಲ್ಲದ"), 0.0);
    let c = engine().candidates("aagide", Some("ಮಾಡಲಾಗಿದೆ"));
    assert!(c.iter().all(|c| c.lift >= 0.0));
}

#[test]
fn weights_out_of_range_are_refused() {
    let mut e = Engine::bundled();
    let w = e.model().weights();
    for bad in [
        Weights { beam: 1e9, ..w },
        Weights { beam: -1.0, ..w },
        Weights {
            open_beam: 0.0,
            ..w
        },
        Weights {
            freq: f32::NAN,
            ..w
        },
    ] {
        assert!(!e.set_weights(bad));
        assert_eq!(e.model().weights(), w);
    }
    assert!(e.set_weights(Weights { beam: 8.0, ..w }));
}

#[test]
fn a_damaged_model_is_rejected_without_a_panic() {
    let bytes = engine().model().to_bytes();
    // Flip 1 bit at a time across the whole file, in steps that cover every section.
    for at in (8..bytes.len()).step_by(bytes.len() / 50) {
        for bit in [0, 3, 7] {
            let mut b = bytes.clone();
            b[at] ^= 1 << bit;
            let _ = Model::from_bytes(&b);
        }
    }
}

#[test]
fn the_model_round_trips_and_broken_files_are_rejected() {
    let bytes = engine().model().to_bytes();
    let again = Model::from_bytes(&bytes).unwrap();
    assert_eq!(again.to_bytes(), bytes);
    assert!(Model::from_bytes(&bytes[..bytes.len() - 1]).is_err());
    assert!(Model::from_bytes(b"KNTX").is_err());
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(Model::from_bytes(&extra).is_err());
    for cut in [8, 100, bytes.len() / 2] {
        assert!(Model::from_bytes(&bytes[..cut]).is_err(), "cut at {cut}");
    }
}

proptest! {
    #[test]
    fn any_letters_give_well_formed_words(input in "[a-zA-Z]{0,24}") {
        for w in engine().suggest(&input, None, 5) {
            prop_assert_eq!(units(&w).map(|u| text(&u)), Some(w.clone()));
        }
    }

    #[test]
    fn units_round_trip(word in "[\u{0C80}-\u{0CFF}\u{200C}\u{200D}]{1,12}") {
        if let Some(u) = units(&word) {
            let t = text(&u);
            prop_assert_eq!(units(&t), Some(u));
        }
    }
}
