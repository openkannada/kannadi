//! Cross-checks ISO 15919 output against vidyut-lipi, an independent MIT-licensed
//! transliterator, on well-formed words where both crates define the same mapping.

use kannadi::{to_latin, Scheme};
use proptest::prelude::*;
use vidyut_lipi::{Lipika, Scheme as VScheme};

/// Well-formed words of consonant clusters and independent vowels. Nukta forms are left
/// out: this crate passes ಕ಼ and similar through, where vidyut-lipi uses Urdu letters (q).
fn word() -> impl Strategy<Value = String> {
    let consonants: Vec<char> = "ಕಖಗಘಙಚಛಜಝಞಟಠಡಢಣತಥದಧನಪಫಬಭಮಯರಱಲಳೞವಶಷಸಹ".chars().collect();
    let signs: Vec<char> = "ಾಿೀುೂೃೄೢೣೆೇೈೊೋೌ".chars().collect();
    // Consonants that take a virama exclude ಱ: see `known_difference`.
    let heads: Vec<char> = consonants.iter().copied().filter(|&c| c != 'ಱ').collect();
    let syllable = (
        prop::collection::vec(prop::sample::select(heads), 0..3),
        prop::sample::select(consonants),
        prop::option::of(prop::sample::select(signs)),
        prop::option::of(prop::sample::select(vec!['ಂ', 'ಃ'])),
    )
        .prop_map(|(head, last, sign, modifier)| {
            let mut s = String::new();
            for c in head {
                s.push(c);
                s.push('್');
            }
            s.push(last);
            s.extend(sign);
            s.extend(modifier);
            s
        });
    let vowels: Vec<char> = "ಅಆಇಈಉಊಋೠಌೡಎಏಐಒಓಔ".chars().collect();
    let vowel = (prop::sample::select(vowels), prop::option::of(Just('ಂ')))
        .prop_map(|(v, m)| v.to_string() + &m.map(String::from).unwrap_or_default());
    prop::collection::vec(prop_oneof![3 => syllable, 1 => vowel], 1..6).prop_map(|v| v.concat())
}

/// Dead ಱ (ಱ್) is the one known difference: vidyut-lipi writes r̆ after the Marathi
/// eyelash ra, and this crate writes ṟ as ISO 15919 does for Dravidian RRA.
fn known_difference(kannada: &str) -> bool {
    kannada.contains("ಱ್")
}

#[test]
fn agrees_on_fixture_words() {
    let mut lipika = Lipika::new();
    for word in [
        "ಕನ್ನಡ",
        "ಬೆಂಗಳೂರು",
        "ಕ್ಷೇತ್ರ",
        "ಸಂಸ್ಕೃತ",
        "ಅಱಿವು",
        "ಮೞೆ",
        "ದುಃಖ",
        "ಏಳು",
    ] {
        assert_eq!(
            to_latin(word, Scheme::Iso15919),
            lipika.transliterate(word, VScheme::Kannada, VScheme::Iso15919),
            "{word}"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(5000))]

    #[test]
    fn agrees_with_vidyut_lipi_on_iso_15919(s in word()) {
        let ours = to_latin(&s, Scheme::Iso15919);
        prop_assert!(!known_difference(&s), "generator produced dead ಱ: {}", s);
        let mut lipika = Lipika::new();
        let theirs = lipika.transliterate(&s, VScheme::Kannada, VScheme::Iso15919);
        prop_assert_eq!(ours, theirs, "{}", s);
    }
}
