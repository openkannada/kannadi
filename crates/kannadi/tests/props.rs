use kannadi::chars::{class, Class};
use kannadi::{aksharas, from_latin, modernize, normalize, numerals, split, to_latin, Scheme};
use proptest::prelude::*;

/// Every assigned character in the Kannada block except digits, which romanize to ASCII.
fn kannada_chars() -> Vec<char> {
    ('\u{0C80}'..='\u{0CFF}')
        .filter(|&c| class(c) != Class::Digit)
        .filter(|&c| is_assigned(c))
        .collect()
}

fn is_assigned(c: char) -> bool {
    !matches!(
        c as u32,
        0x0C8D | 0x0C91 | 0x0CA9 | 0x0CB4 | 0x0CBA | 0x0CBB | 0x0CC5 | 0x0CC9
            | 0x0CCE..=0x0CD4 | 0x0CD7..=0x0CDC | 0x0CDF | 0x0CE4 | 0x0CE5 | 0x0CF0
            | 0x0CF4..=0x0CFF
    )
}

/// Any sequence of Kannada characters, spaces, joiners, digits and ASCII punctuation,
/// including the characters schemes use as separators and escapes.
fn kannada_text() -> impl Strategy<Value = String> {
    let mut pool = kannada_chars();
    pool.extend([
        ' ', ' ', '\u{200C}', '\u{200D}', ',', '.', '?', '0', '7', '\n',
    ]);
    // Characters that schemes use in tokens, separators or escapes.
    pool.extend([
        ':', '\'', '\u{2019}', '_', '^', '~', '@', '\\', '\u{0951}', '\u{0952}', '\u{1CD4}',
    ]);
    prop::collection::vec(prop::sample::select(pool), 0..40).prop_map(|v| v.into_iter().collect())
}

/// Well-formed words: consonant clusters and vowels with signs and modifiers.
fn kannada_word() -> impl Strategy<Value = String> {
    let consonants: Vec<char> = ('\u{0C95}'..='\u{0CB9}')
        .chain(['\u{0CDE}'])
        .filter(|&c| class(c) == Class::Consonant)
        .collect();
    let vowels: Vec<char> = ('\u{0C85}'..='\u{0CE1}')
        .filter(|&c| class(c) == Class::Vowel)
        .collect();
    let signs: Vec<char> = "ಾಿೀುೂೃೄೆೇೈೊೋೌೢೣ".chars().collect();
    let cons = prop::sample::select(consonants);
    let cluster = (
        prop::collection::vec(cons.clone(), 0..3),
        cons,
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
    let vowel = (prop::sample::select(vowels), prop::option::of(Just('ಂ'))).prop_map(|(v, m)| {
        let mut s = String::from(v);
        s.extend(m);
        s
    });
    prop::collection::vec(prop_oneof![3 => cluster, 1 => vowel], 1..6).prop_map(|v| v.concat())
}

/// 2,000 cases by default; set PROPTEST_CASES for a longer run.
fn cases() -> u32 {
    std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(2000)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(cases()))]

    #[test]
    fn roundtrip_any_kannada_text(s in kannada_text()) {
        for scheme in [Scheme::Iso15919, Scheme::Iast, Scheme::Baraha] {
            let latin = to_latin(&s, scheme);
            prop_assert_eq!(from_latin(&latin, scheme), s.clone(), "{} via {:?}", latin, scheme);
        }
    }

    #[test]
    fn roundtrip_words(s in kannada_word()) {
        for scheme in [Scheme::Iso15919, Scheme::Iast, Scheme::Baraha] {
            let latin = to_latin(&s, scheme);
            prop_assert!(!latin.chars().any(kannadi::chars::is_kannada), "{} left Kannada in {}", s, latin);
            prop_assert_eq!(from_latin(&latin, scheme), s.clone(), "{} via {:?}", latin, scheme);
        }
    }

    #[test]
    fn latin_to_latin_is_stable(s in kannada_word()) {
        let iso = to_latin(&s, Scheme::Iso15919);
        let baraha = kannadi::transliterate(&iso, Scheme::Iso15919, Scheme::Baraha);
        prop_assert_eq!(kannadi::transliterate(&baraha, Scheme::Baraha, Scheme::Iso15919), iso);
    }

    #[test]
    fn segments_cover_input_exactly(s in any::<String>()) {
        let parts = split(&s);
        prop_assert_eq!(parts.concat(), s.clone());
        prop_assert!(parts.iter().all(|p| !p.is_empty()));
    }

    #[test]
    fn segments_cover_kannada_exactly(s in kannada_text()) {
        let parts = split(&s);
        prop_assert_eq!(parts.concat(), s.clone());
        for part in &parts {
            prop_assert_eq!(split(part), vec![*part], "segment {:?} of {:?}", part, s);
        }
    }

    #[test]
    fn words_segment_into_syllables(s in kannada_word()) {
        prop_assert!(aksharas(&s).all(|a| a.is_syllable()), "{:?}", split(&s));
    }

    #[test]
    fn normalize_is_idempotent(s in kannada_text()) {
        let once = normalize(&s);
        prop_assert_eq!(normalize(&once), once);
    }

    #[test]
    fn modernize_removes_old_letters(s in kannada_text()) {
        let m = modernize(&s);
        prop_assert!(!kannadi::has_old_kannada(&m));
        prop_assert_eq!(modernize(&m), m);
    }

    #[test]
    fn numbers_roundtrip(n in any::<u64>()) {
        prop_assert_eq!(numerals::parse(&numerals::format(n)), Some(n));
        prop_assert_eq!(numerals::to_ascii_digits(&numerals::format(n)), n.to_string());
    }
}
