use kannadi::chars::{class, Class};
use kannadi::legacy::{
    from_unicode, nudi_syllable_starts, nudi_to_unicode, syllable_starts, to_unicode,
    unicode_to_nudi, Layout, Options,
};
use kannadi::normalize::{is_normalized, normalize_as, Form};
use kannadi::{
    aksharas, from_latin, modernize, normalize, numerals, split, to_latin, typos, Scheme,
};
use proptest::prelude::*;
use unicode_normalization::UnicodeNormalization;

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
    let vowels = ('\u{0C85}'..='\u{0CE1}')
        .filter(|&c| class(c) == Class::Vowel)
        .collect();
    word(vowels, "ಾಿೀುೂೃೄೆೇೈೊೋೌೢೣ", vec!['ಂ'])
}

/// Well-formed words using only the letters and signs the Nudi layout has glyphs
/// for, including clusters that end in a virama.
fn nudi_word() -> impl Strategy<Value = String> {
    let vowels = ('\u{0C85}'..='\u{0CE1}')
        .filter(|&c| class(c) == Class::Vowel && !matches!(c, 'ಌ' | 'ೡ'))
        .collect();
    word(vowels, "ಾಿೀುೂೃೆೇೈೊೋೌ್", vec!['ಂ', 'ಃ'])
}

/// Well-formed words using only what the Akruti layout has glyphs for: no ಙ, ಱ
/// or ೞ, ಝ only as a subscript, and no ಞ with ಿ or ೀ.
fn akruti_word() -> impl Strategy<Value = String> {
    let vowels = ('\u{0C85}'..='\u{0CE1}')
        .filter(|&c| class(c) == Class::Vowel && !matches!(c, 'ಌ' | 'ೡ'))
        .collect();
    let consonants = consonants()
        .into_iter()
        .filter(|c| !matches!(c, 'ಙ' | 'ಱ' | 'ೞ'))
        .collect();
    word_of(consonants, vowels, "ಾಿೀುೂೃೆೇೈೊೋೌ್", vec!['ಂ', 'ಃ']).prop_filter(
        "a letter Akruti has no glyph for",
        |w| {
            aksharas(w).all(|a| {
                // The consonant that carries the syllable follows an arkavattu ರ್.
                let text = a
                    .text
                    .strip_prefix("ರ್")
                    .filter(|t| !t.is_empty())
                    .unwrap_or(a.text);
                match text.chars().next() {
                    Some('ಝ') => false,
                    Some('ಞ') => !text.contains(['ಿ', 'ೀ']),
                    _ => true,
                }
            })
        },
    )
}

fn consonants() -> Vec<char> {
    ('\u{0C95}'..='\u{0CB9}')
        .chain(['\u{0CDE}'])
        .filter(|&c| class(c) == Class::Consonant)
        .collect()
}

fn word(vowels: Vec<char>, signs: &str, vowel_marks: Vec<char>) -> impl Strategy<Value = String> {
    word_of(consonants(), vowels, signs, vowel_marks)
}

fn word_of(
    consonants: Vec<char>,
    vowels: Vec<char>,
    signs: &str,
    vowel_marks: Vec<char>,
) -> impl Strategy<Value = String> {
    let signs: Vec<char> = signs.chars().collect();
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
            if sign != Some('್') {
                s.extend(modifier);
            }
            s
        });
    let vowel = (
        prop::sample::select(vowels),
        prop::option::of(prop::sample::select(vowel_marks)),
    )
        .prop_map(|(v, m)| {
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
    fn every_form_is_idempotent_and_includes_the_one_before(s in kannada_text()) {
        let mut before: Option<String> = None;
        for form in Form::ALL {
            let once = normalize_as(&s, form);
            prop_assert_eq!(normalize_as(&once, form), once.clone(), "{:?}", form);
            prop_assert!(is_normalized(&once, form), "{:?}", form);
            if let Some(prev) = before {
                prop_assert_eq!(normalize_as(&prev, form), once.clone(), "{:?}", form);
            }
            before = Some(once);
        }
    }

    #[test]
    fn modernize_removes_old_letters(s in kannada_text()) {
        let m = modernize(&s);
        prop_assert!(!kannadi::has_old_kannada(&m));
        prop_assert_eq!(modernize(&m), m);
    }

    #[test]
    fn nudi_roundtrip_words(
        words in prop::collection::vec(nudi_word(), 1..4),
        gaps in prop::collection::vec(prop::sample::select(vec![" ", ", ", "। ", " 7 ", "\n", " ಽ "]), 4),
    ) {
        let mut text = String::new();
        for (word, gap) in words.iter().zip(&gaps) {
            text.push_str(word);
            text.push_str(gap);
        }
        let encoded = unicode_to_nudi(&text);
        prop_assert!(encoded.is_clean(), "{:?}", encoded);
        let back = nudi_to_unicode(&encoded.text, &Options::default());
        prop_assert!(back.is_clean(), "{:?} from {:?}", back, encoded.text);
        prop_assert_eq!(back.text, text);
    }

    #[test]
    fn akruti_roundtrip_words(
        words in prop::collection::vec(akruti_word(), 1..4),
        number in "[೧-೯]{0,3}",
    ) {
        let text = format!("{} {number}", words.join(" "));
        let encoded = from_unicode(&text, Layout::Akruti);
        prop_assert!(encoded.is_clean(), "{:?}", encoded);
        let back = to_unicode(&encoded.text, Layout::Akruti, &Options::default());
        prop_assert!(back.is_clean(), "{:?} from {:?}", back, encoded.text);
        prop_assert_eq!(back.text, text);
    }

    /// Clean conversions are canonical; malformed ones are reported as they are.
    #[test]
    fn glyph_codes_convert_to_canonical_text(bytes in prop::collection::vec(any::<u8>(), 0..60)) {
        let legacy = kannadi::legacy::decode_windows_1252(&bytes);
        for layout in Layout::ALL {
            let converted = to_unicode(&legacy, layout, &Options::default());
            if converted.is_clean() {
                let text = converted.text;
                prop_assert!(is_normalized(&text, Form::Canonical), "{}: {:?}", layout, text);
            }
        }
    }

    #[test]
    fn clean_encodings_roundtrip(s in kannada_text()) {
        for layout in Layout::ALL {
            let encoded = from_unicode(&s, layout);
            if encoded.is_clean() {
                let keep_zeros = Options { zero_as_anusvara: false, ..Options::default() };
                let back = to_unicode(&encoded.text, layout, &keep_zeros);
                prop_assert!(back.is_clean(), "{}: {:?} from {:?}", layout, back, encoded.text);
                let expected: String = numerals::to_ascii_digits(&s).nfc().collect();
                prop_assert_eq!(back.text, expected, "{}", layout);
            }
        }
    }

    #[test]
    fn nudi_converts_the_same_split_at_syllable_starts(
        words in prop::collection::vec(nudi_word(), 1..4),
        cuts in prop::collection::vec(any::<prop::sample::Index>(), 0..6),
    ) {
        let legacy = unicode_to_nudi(&words.join(" ")).text;
        let starts = nudi_syllable_starts(&legacy);
        let mut at: Vec<usize> = cuts.iter().map(|c| starts[c.index(starts.len())]).collect();
        at.extend([0, legacy.len()]);
        at.sort_unstable();
        at.dedup();
        let options = Options { zero_as_anusvara: false, ..Options::default() };
        let pieces: String = at
            .windows(2)
            .map(|w| nudi_to_unicode(&legacy[w[0]..w[1]], &options).text)
            .collect();
        prop_assert_eq!(pieces, nudi_to_unicode(&legacy, &options).text);
    }

    #[test]
    fn converts_the_same_split_anywhere_it_allows(
        bytes in prop::collection::vec(any::<u8>(), 0..60),
        cuts in prop::collection::vec(any::<prop::sample::Index>(), 0..6),
    ) {
        let legacy = kannadi::legacy::decode_windows_1252(&bytes);
        for layout in Layout::ALL {
            let starts = syllable_starts(&legacy, layout);
            let mut at: Vec<usize> = match starts.len() {
                0 => Vec::new(),
                n => cuts.iter().map(|c| starts[c.index(n)]).collect(),
            };
            at.extend([0, legacy.len()]);
            at.sort_unstable();
            at.dedup();
            let options = Options { zero_as_anusvara: false, ..Options::default() };
            let whole = to_unicode(&legacy, layout, &options);
            let pieces: Vec<_> = at.windows(2).map(|w| to_unicode(&legacy[w[0]..w[1]], layout, &options)).collect();
            prop_assert_eq!(pieces.iter().map(|p| p.text.as_str()).collect::<String>(), whole.text, "{}", layout);
            prop_assert_eq!(pieces.iter().map(|p| p.unknown.len()).sum::<usize>(), whole.unknown.len(), "{}", layout);
            prop_assert_eq!(pieces.iter().map(|p| p.malformed.len()).sum::<usize>(), whole.malformed.len(), "{}", layout);
        }
    }

    #[test]
    fn reports_point_at_their_characters(s in kannada_text()) {
        for layout in Layout::ALL {
            let encoded = from_unicode(&s, layout);
            for u in &encoded.unencodable {
                prop_assert_eq!(s[u.offset..].chars().next(), Some(u.character));
            }
        }
    }

    #[test]
    fn typo_fixes_are_ordered_and_leave_no_typos(
        parts in prop::collection::vec(
            prop::sample::select(vec!["ರ", "್", "ಋ", "ೃ", "ಕ", "ಾ", "ಂ", "\u{200C}", "\u{200D}", " "]),
            0..24,
        )
    ) {
        let text: String = parts.concat();
        let found = typos::find(&text);
        let mut fixed = String::new();
        let mut end = 0;
        for t in &found {
            prop_assert!(t.range.start >= end && t.range.start < t.range.end, "{:?}", text);
            prop_assert!(text.is_char_boundary(t.range.start) && text.is_char_boundary(t.range.end));
            let spelled = match t.rule {
                typos::Rule::RaVocalicR => "ರೃ",
                typos::Rule::RaViramaVocalicR => "ರ್ಋ",
                _ => unreachable!(),
            };
            prop_assert_eq!(&text[t.range.clone()], spelled);
            prop_assert_eq!(t.replacement, "ಋ");
            fixed.push_str(&text[end..t.range.start]);
            fixed.push_str(t.replacement);
            end = t.range.end;
        }
        fixed.push_str(&text[end..]);
        prop_assert!(typos::find(&fixed).is_empty(), "{:?} gave {:?}", text, fixed);
        // Every ರ್ಋ that is a word of its own, with no joiner or mark around it, is found.
        for word in text.split(' ') {
            if word == "ರ್ಋ" {
                prop_assert!(found.iter().any(|t| t.rule == typos::Rule::RaViramaVocalicR), "{:?}", text);
            }
        }
    }

    #[test]
    fn numbers_roundtrip(n in any::<u64>()) {
        prop_assert_eq!(numerals::parse(&numerals::format(n)), Some(n));
        prop_assert_eq!(numerals::to_ascii_digits(&numerals::format(n)), n.to_string());
    }
}
