//! Normalisation and modernisation.

use unicode_normalization::UnicodeNormalization;

use crate::chars::{class, is_consonant, Class, LLLA, NAKAARA_POLLU, RRA, VIRAMA, ZWNJ};

/// Normalises Kannada text so that equal-looking text compares equal.
///
/// Steps, in order:
///
/// 1. Unicode NFC, which joins two-part vowel signs: ಿ + ೕ becomes ೀ,
///    ೆ + ೂ becomes ೊ, ೆ + ೂ + ೕ becomes ೋ.
/// 2. Look-alike vowel letters that Unicode says not to build from parts:
///    ಉ + ಾ becomes ಊ, ಋ + ಾ becomes ೠ.
/// 3. A combining mark repeated twice in a row (for example ್್ or ಂಂ) is kept once.
/// 4. NFC again, in case step 3 made a new composable pair.
///
/// Text outside the Kannada block is only touched by NFC. For text from legacy fonts
/// or OCR, see also [`fix_zero_as_anusvara`].
///
/// ```
/// assert_eq!(kannadi::normalize("ಕಿ\u{0CD5}ರ್ತಿ"), "ಕೀರ್ತಿ");
/// ```
pub fn normalize(text: &str) -> String {
    let composed: Vec<char> = text.nfc().collect();
    let mut out: Vec<char> = Vec::with_capacity(composed.len());
    for c in composed {
        let prev = out.last().copied();
        match (prev, c) {
            (Some('ಉ'), 'ಾ') => *out.last_mut().unwrap() = 'ಊ',
            (Some('ಋ'), 'ಾ') => *out.last_mut().unwrap() = 'ೠ',
            (Some(p), c) if p == c && class(c).is_mark() => {}
            _ => out.push(c),
        }
    }
    out.into_iter().nfc().collect()
}

fn is_letter_or_mark(c: char) -> bool {
    let c = class(c);
    matches!(c, Class::Vowel | Class::Consonant) || c.is_mark()
}

/// Replaces a Kannada digit zero (೦) written right after a letter or sign with an
/// anusvara (ಂ). Legacy fonts and OCR often confuse the two.
///
/// Use this on converted or scanned text only: a number written directly after a
/// word, such as ಕೊಠಡಿ೦೧, is changed too.
///
/// ```
/// assert_eq!(kannadi::fix_zero_as_anusvara("ಬೆ೦ಗಳೂರು"), "ಬೆಂಗಳೂರು");
/// assert_eq!(kannadi::fix_zero_as_anusvara("೧೦೦"), "೧೦೦");
/// ```
pub fn fix_zero_as_anusvara(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev: Option<char> = None;
    for c in text.chars() {
        match prev {
            Some(p) if c == '೦' && is_letter_or_mark(p) => out.push('ಂ'),
            _ => out.push(c),
        }
        prev = Some(c);
    }
    out
}

/// Maps Old Kannada letters to their modern equivalents, for search and indexing.
///
/// - ೞ (LLLA) becomes ಳ
/// - ಱ (RRA) becomes ರ
/// - ೝ (nakaara pollu) becomes ನ್, followed by ZWNJ when a consonant comes next,
///   so it does not join that consonant into a conjunct
///
/// ```
/// assert_eq!(kannadi::modernize("ಅಱಿವು ಬಾೞ್"), "ಅರಿವು ಬಾಳ್");
/// assert_eq!(kannadi::modernize("ಅವೝಕಂಡ"), "ಅವನ್\u{200C}ಕಂಡ");
/// ```
pub fn modernize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            LLLA => out.push('ಳ'),
            RRA => out.push('ರ'),
            NAKAARA_POLLU => {
                out.push('ನ');
                out.push(VIRAMA);
                if chars
                    .peek()
                    .is_some_and(|&n| is_consonant(n) || n == NAKAARA_POLLU)
                {
                    out.push(ZWNJ);
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// True if `text` contains an Old Kannada letter that [`modernize`] would change.
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
        assert_eq!(fix_zero_as_anusvara("ಬೆ೦ಗಳೂರು"), "ಬೆಂಗಳೂರು");
        assert_eq!(fix_zero_as_anusvara("ಅ೦ಗಡಿ"), "ಅಂಗಡಿ");
        assert_eq!(fix_zero_as_anusvara("೧೦೦"), "೧೦೦");
        assert_eq!(fix_zero_as_anusvara("೦"), "೦");
        assert_eq!(fix_zero_as_anusvara("ಕ ೦"), "ಕ ೦");
        assert_eq!(fix_zero_as_anusvara("ಕ೦೦"), "ಕಂ೦");
    }

    #[test]
    fn normalize_leaves_numbers_after_words_alone() {
        assert_eq!(normalize("ಕೊಠಡಿ೦೧"), "ಕೊಠಡಿ೦೧");
        assert_eq!(normalize("ಬೆ೦ಗಳೂರು"), "ಬೆ೦ಗಳೂರು");
    }

    #[test]
    fn repeated_marks_collapse() {
        assert_eq!(normalize("ಕ್್ಕ"), "ಕ್ಕ");
        assert_eq!(normalize("ಕಂಂ"), "ಕಂ");
        assert_eq!(normalize("ಕಕ"), "ಕಕ");
    }

    #[test]
    fn modernize_maps_old_letters() {
        assert_eq!(modernize("ಕೞ್ತೆ"), "ಕಳ್ತೆ");
        assert_eq!(modernize("ಅಱಿ"), "ಅರಿ");
        assert_eq!(modernize("ಅವೝ"), "ಅವನ್");
        assert_eq!(modernize("ಅವೝ ಕಂಡ"), "ಅವನ್ ಕಂಡ");
        assert_eq!(modernize("ಅವೝಕಂಡ"), "ಅವನ್\u{200C}ಕಂಡ");
        assert_eq!(modernize("ೝೝ"), "ನ್\u{200C}ನ್");
        assert_eq!(modernize("plain ಕನ್ನಡ"), "plain ಕನ್ನಡ");
        assert!(has_old_kannada("ಅಱಿ"));
        assert!(!has_old_kannada("ಅರಿ"));
    }
}
