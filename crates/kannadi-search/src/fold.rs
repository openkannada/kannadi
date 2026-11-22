//! The search key of 1 word: spellings a reader treats as the same word become 1 string.
//!
//! [`fold`] applies the search form of the normalization profile (`NORMALIZATION.md`
//! in kannadi): canonical joiners and vowel signs, legacy repairs such as ೦ typed
//! for ಂ, Old Kannada letters, joiners and the nukta. It then folds what the
//! profile leaves to search tools:
//!
//! - a nasal consonant and virama before a consonant of its own group becomes the
//!   anusvara, which is how Kannada usually spells it: ಸಮ್ಬನ್ಧ becomes ಸಂಬಂಧ;
//! - Kannada digits become ASCII digits: ೨೦೨೬ becomes 2026;
//! - letters of other scripts are lowercased.

use kannadi::chars::{ANUSVARA, VIRAMA};
use kannadi::normalize::{normalize_as, Form};

/// The search key of `word`. Folding a key again changes nothing.
///
/// ```
/// use kannadi_search::fold::fold;
///
/// assert_eq!(fold("ಕನ್\u{200C}ನಡ"), "ಕನ್ನಡ");
/// assert_eq!(fold("ಬಾೞ್"), "ಬಾಳ್");
/// assert_eq!(fold("ಬೆ೦ಗಳೂರು"), "ಬೆಂಗಳೂರು");
/// assert_eq!(fold("ಸಮ್ಬನ್ಧ"), "ಸಂಬಂಧ");
/// assert_eq!(fold("೨೦೨೬"), "2026");
/// assert_eq!(fold("Bengaluru"), "bengaluru");
/// ```
pub fn fold(word: &str) -> String {
    let normalized = normalize_as(word, Form::Search);
    let chars: Vec<char> = normalized.chars().collect();
    let mut out = String::with_capacity(normalized.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if chars.get(i + 1) == Some(&VIRAMA)
            && chars.get(i + 2).is_some_and(|&next| is_homorganic(c, next))
        {
            out.push(ANUSVARA);
            i += 2;
            continue;
        }
        match c {
            '\u{0CE6}'..='\u{0CEF}' => out.push(char::from(b'0' + (c as u32 - 0x0CE6) as u8)),
            c if c.is_uppercase() => out.extend(c.to_lowercase()),
            c => out.push(c),
        }
        i += 1;
    }
    if out.contains("\u{0C82}\u{0C82}") {
        // ಂ next to a folded nasal (ಕಂಞ್ಚ): the profile keeps a repeated mark once.
        return normalize_as(&out, Form::Search);
    }
    out
}

/// True when `nasal` is the nasal of the consonant group (varga) of `next`:
/// ಙ before ಕ to ಘ, ಞ before ಚ to ಝ, ಣ before ಟ to ಢ, ನ before ತ to ಧ, ಮ before ಪ to ಭ.
/// A doubled nasal, as in ಕನ್ನಡ, is not in its own group's stops and stays.
fn is_homorganic(nasal: char, next: char) -> bool {
    match nasal {
        'ಙ' => ('ಕ'..='ಘ').contains(&next),
        'ಞ' => ('ಚ'..='ಝ').contains(&next),
        'ಣ' => ('ಟ'..='ಢ').contains(&next),
        'ನ' => ('ತ'..='ಧ').contains(&next),
        'ಮ' => ('ಪ'..='ಭ').contains(&next),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_search_form_applies() {
        assert_eq!(fold("ಆಫ್\u{200C}"), "ಆಫ್");
        assert_eq!(fold("ರಾಜ್\u{200D}ಕುಮಾರ್"), fold("ರಾಜ್\u{200C}ಕುಮಾರ್"));
        assert_eq!(fold("ರಾಜ್\u{200D}ಕುಮಾರ್"), "ರಾಜ್ಕುಮಾರ್");
        assert_eq!(fold("ಜ಼ೀರೋ"), "ಜೀರೋ");
        assert_eq!(fold("ಕಿ\u{0CD5}ರ್ತಿ"), "ಕೀರ್ತಿ");
        assert_eq!(fold("ಸಕರ್ಾರ"), "ಸರ್ಕಾರ");
        assert_eq!(fold("ಹೊoದಿದೆ"), "ಹೊಂದಿದೆ");
    }

    #[test]
    fn nasal_and_virama_before_its_group_is_the_anusvara() {
        for (nasal, group) in [
            ('ಙ', 'ಕ'..='ಘ'),
            ('ಞ', 'ಚ'..='ಝ'),
            ('ಣ', 'ಟ'..='ಢ'),
            ('ನ', 'ತ'..='ಧ'),
            ('ಮ', 'ಪ'..='ಭ'),
        ] {
            for stop in group {
                let word = format!("ಅ{nasal}{VIRAMA}{stop}");
                assert_eq!(fold(&word), format!("ಅಂ{stop}"), "{word}");
            }
        }
        assert_eq!(fold("ಸನ್ತೋಷ"), fold("ಸಂತೋಷ"));
        assert_eq!(fold("ಗಙ್ಗಾ"), "ಗಂಗಾ");
    }

    #[test]
    fn other_clusters_with_a_nasal_stay() {
        for word in ["ಕನ್ನಡ", "ಅಮ್ಮ", "ಅಣ್ಣ", "ಜನ್ಮ", "ಬ್ರಹ್ಮ", "ತಾಮ್ರ", "ಅನ್ಯ", "ಮ್ಯೂಸಿಯಂ"] {
            assert_eq!(fold(word), word);
        }
        assert_eq!(fold("ನ್ಕ"), "ನ್ಕ");
    }

    #[test]
    fn digits_and_case() {
        assert_eq!(fold("೧೯೯೦ರಲ್ಲಿ"), "1990ರಲ್ಲಿ");
        assert_eq!(fold("ಕೊಠಡಿ೦೧"), "ಕೊಠಡಿ01");
        assert_eq!(fold("ÉCOLE"), "école");
        assert_eq!(fold("İ"), "i\u{307}");
    }

    #[test]
    fn folding_is_idempotent() {
        assert_eq!(fold("ಕಂಞ್ಚ"), "ಕಂಚ");
        for word in ["ಕನ್\u{200C}ನಡ", "ಸಮ್ಬನ್ಧ", "ಬೆ೦ಗಳೂರು", "ಸಕರ್ಾರ", "ÉCOLE", "ಙ್ಙ"] {
            let once = fold(word);
            assert_eq!(fold(&once), once, "{word}");
        }
    }
}
