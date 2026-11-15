//! Normalisation and modernisation.

use unicode_normalization::UnicodeNormalization;

use crate::chars::{class, Class, LLLA, NAKAARA_POLLU, RRA, VIRAMA};

/// Normalises Kannada text so that equal-looking text compares equal.
///
/// Steps, in order:
///
/// 1. Unicode NFC, which joins two-part vowel signs: ಿ + ೕ becomes ೀ,
///    ೆ + ೂ becomes ೊ, ೆ + ೂ + ೕ becomes ೋ.
/// 2. Look-alike vowel letters that Unicode says not to build from parts:
///    ಉ + ಾ becomes ಊ, ಋ + ಾ becomes ೠ.
/// 3. A Kannada digit zero (೦) right after a letter or sign becomes an anusvara (ಂ).
///    Legacy fonts and OCR often confuse the two; a digit never follows a letter
///    inside a Kannada word.
/// 4. A combining mark repeated twice in a row (for example ್್ or ಂಂ) is kept once.
/// 5. NFC again, in case step 4 made a new composable pair.
///
/// Text outside the Kannada block is only touched by NFC.
///
/// ```
/// assert_eq!(kannadi::normalize("ಕಿ\u{0CD5}ರ್ತಿ"), "ಕೀರ್ತಿ");
/// assert_eq!(kannadi::normalize("ಕ೦ದ"), "ಕಂದ");
/// ```
pub fn normalize(text: &str) -> String {
    let composed: Vec<char> = text.nfc().collect();
    let mut out: Vec<char> = Vec::with_capacity(composed.len());
    for c in composed {
        let prev = out.last().copied();
        match (prev, c) {
            (Some('ಉ'), 'ಾ') => *out.last_mut().unwrap() = 'ಊ',
            (Some('ಋ'), 'ಾ') => *out.last_mut().unwrap() = 'ೠ',
            (Some(p), '೦') if is_letter_or_mark(p) => out.push('ಂ'),
            (Some(p), c) if p == c && is_mark(c) => {}
            _ => out.push(c),
        }
    }
    out.into_iter().nfc().collect()
}

fn is_mark(c: char) -> bool {
    matches!(
        class(c),
        Class::VowelSign | Class::Virama | Class::Nukta | Class::Modifier
    )
}

fn is_letter_or_mark(c: char) -> bool {
    matches!(class(c), Class::Vowel | Class::Consonant) || is_mark(c)
}

/// Maps Old Kannada letters to their modern equivalents, for search and indexing.
///
/// - ೞ (LLLA) becomes ಳ
/// - ಱ (RRA) becomes ರ
/// - ೝ (nakaara pollu) becomes ನ್
///
/// ```
/// assert_eq!(kannadi::modernise("ಅಱಿವು ಬಾೞ್"), "ಅರಿವು ಬಾಳ್");
/// ```
pub fn modernise(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            LLLA => out.push('ಳ'),
            RRA => out.push('ರ'),
            NAKAARA_POLLU => {
                out.push('ನ');
                out.push(VIRAMA);
            }
            _ => out.push(c),
        }
    }
    out
}

/// True if `text` contains an Old Kannada letter that [`modernise`] would change.
pub fn has_old_kannada(text: &str) -> bool {
    text.chars()
        .any(|c| matches!(c, LLLA | RRA | NAKAARA_POLLU))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nfc_composes_two_part_vowel_signs() {
        assert_eq!(normalize("ಕೆ\u{0CD5}"), "ಕೇ");
        assert_eq!(normalize("ಕೆ\u{0CC2}"), "ಕೊ");
        assert_eq!(normalize("ಕೆ\u{0CC2}\u{0CD5}"), "ಕೋ");
        assert_eq!(normalize("ಕೆ\u{0CD6}"), "ಕೈ");
    }

    #[test]
    fn look_alike_vowel_letters() {
        assert_eq!(normalize("ಉಾರು"), "ಊರು");
        assert_eq!(normalize("ಋಾ"), "ೠ");
        assert_eq!(normalize("ಕಾ"), "ಕಾ");
    }

    #[test]
    fn zero_after_letter_becomes_anusvara() {
        assert_eq!(normalize("ಬೆ೦ಗಳೂರು"), "ಬೆಂಗಳೂರು");
        assert_eq!(normalize("ಅ೦ಗಡಿ"), "ಅಂಗಡಿ");
        assert_eq!(normalize("೧೦೦"), "೧೦೦");
        assert_eq!(normalize("೦"), "೦");
        assert_eq!(normalize("ಕ ೦"), "ಕ ೦");
    }

    #[test]
    fn repeated_marks_collapse() {
        assert_eq!(normalize("ಕ್್ಕ"), "ಕ್ಕ");
        assert_eq!(normalize("ಕಂಂ"), "ಕಂ");
        assert_eq!(normalize("ಕಕ"), "ಕಕ");
    }

    #[test]
    fn modernise_maps_old_letters() {
        assert_eq!(modernise("ಕೞ್ತೆ"), "ಕಳ್ತೆ");
        assert_eq!(modernise("ಅಱಿ"), "ಅರಿ");
        assert_eq!(modernise("ಅವೝ"), "ಅವನ್");
        assert_eq!(modernise("plain ಕನ್ನಡ"), "plain ಕನ್ನಡ");
        assert!(has_old_kannada("ಅಱಿ"));
        assert!(!has_old_kannada("ಅರಿ"));
    }
}
