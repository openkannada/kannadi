//! Spellings that are well formed but are often typing mistakes in modern Kannada.
//!
//! Each [`Typo`] is a suggestion for a writer to accept or keep. Some of these
//! spellings are correct in other contexts, such as Sanskrit, so they are never
//! changed in bulk.

use std::ops::Range;

use crate::akshara::aksharas;

/// Which rule found a [`Typo`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Rule {
    /// ರ with the vowel sign ೃ (U+0CB0 U+0CC3) at the start of a syllable, where
    /// modern Kannada writes the vowel letter ಋ (U+0C8B): ನೈರೃತ್ಯ for ನೈಋತ್ಯ.
    /// Baraha phonetic typing produces it from `r~ru`. Sanskrit spells r + ṛ
    /// this way (nairṛtya), so it is correct in Sanskrit text.
    RaVocalicR,
    /// ರ್ followed by ಋ (U+0CB0 U+0CCD U+0C8B), where modern Kannada writes ಋ:
    /// ನೈರ್ಋತ್ಯ for ನೈಋತ್ಯ. Nudi typists draw ಋ with an arkavattu after it,
    /// which converts to ರ್ಋ. Sanskrit spells r + ṛ this way (nirṛti), so it
    /// is correct in Sanskrit text.
    RaViramaVocalicR,
}

impl Rule {
    /// A stable kebab-case name, such as `ra-vocalic-r`.
    pub const fn name(self) -> &'static str {
        match self {
            Rule::RaVocalicR => "ra-vocalic-r",
            Rule::RaViramaVocalicR => "ra-virama-vocalic-r",
        }
    }
}

/// A likely typing mistake: replace `range` (byte offsets) with `replacement`.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct Typo {
    /// The text to replace.
    pub range: Range<usize>,
    /// The likely intended spelling.
    pub replacement: &'static str,
    /// The rule that found it.
    pub rule: Rule,
}

const RA_VOCALIC_R: &str = "\u{0CB0}\u{0CC3}";
const RA_VIRAMA: &str = "\u{0CB0}\u{0CCD}";
const VOCALIC_R: &str = "\u{0C8B}";
/// Virama and ZWNJ: an explicit half form that ends its akshara.
const HALF_FORM: &str = "\u{0CCD}\u{200C}";

/// Finds likely typing mistakes in `text`, in text order.
///
/// Only well-formed aksharas are checked; [`crate::Akshara::is_well_formed`]
/// finds the others. The rules:
///
/// - ರೃ at the start of a syllable suggests ಋ, as in ನೈಋತ್ಯ. A ra joined
///   below another consonant (ಕ್ರೃ) or after an explicit half form (ಕ್‌ರೃ, with
///   ZWNJ) is left alone, as the intended spelling is not clear there.
/// - ರ್ as a syllable of its own, followed directly by ಋ, suggests ಋ for both,
///   as in ನೈರ್ಋತ್ಯ. A ರ್ joined to ಋ by ZWNJ or ZWJ, a ರ್ at the end of a
///   cluster (ಗ್ರ್ಋ) or after an explicit half form, and ರ್ before any other
///   vowel letter (ಆರ್ಎನ್ಎ, ಕುರ್ಆನ್) are left alone.
///
/// ```
/// use kannadi::typos::{find, Rule};
///
/// let text = "ನೈರೃತ್ಯ";
/// let found = find(text);
/// assert_eq!(found.len(), 1);
/// assert_eq!(found[0].rule, Rule::RaVocalicR);
/// assert_eq!(&text[found[0].range.clone()], "ರೃ");
/// assert_eq!(found[0].replacement, "ಋ");
/// ```
pub fn find(text: &str) -> Vec<Typo> {
    let mut out = Vec::new();
    let mut pos = 0;
    let mut after_half_form = false;
    let mut iter = aksharas(text).peekable();
    while let Some(a) = iter.next() {
        if !after_half_form && a.text.starts_with(RA_VOCALIC_R) && a.is_well_formed() {
            out.push(Typo {
                range: pos..pos + RA_VOCALIC_R.len(),
                replacement: VOCALIC_R,
                rule: Rule::RaVocalicR,
            });
        }
        if !after_half_form
            && a.text == RA_VIRAMA
            && iter
                .peek()
                .is_some_and(|next| next.text.starts_with(VOCALIC_R) && next.is_well_formed())
        {
            out.push(Typo {
                range: pos..pos + RA_VIRAMA.len() + VOCALIC_R.len(),
                replacement: VOCALIC_R,
                rule: Rule::RaViramaVocalicR,
            });
        }
        after_half_form = a.text.ends_with(HALF_FORM);
        pos += a.text.len();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{transliterate, Scheme};

    fn found(text: &str) -> Vec<&str> {
        find(text).iter().map(|t| &text[t.range.clone()]).collect()
    }

    fn fixed(text: &str) -> String {
        let mut out = text.to_string();
        for t in find(text).iter().rev() {
            out.replace_range(t.range.clone(), t.replacement);
        }
        out
    }

    #[test]
    fn ra_with_vocalic_r_sign_suggests_the_vowel_letter() {
        assert_eq!(fixed("ನೈರೃತ್ಯ"), "ನೈಋತ್ಯ");
        assert_eq!(fixed("ನಿರೃತಿ"), "ನಿಋತಿ");
        assert_eq!(fixed("ರೃಷಿ"), "ಋಷಿ");
        assert_eq!(fixed("ರೃಂ"), "ಋಂ");
        assert_eq!(fixed("ರೃ ರೃ"), "ಋ ಋ");
        assert_eq!(fixed("ರೃರೃ"), "ಋಋ");
    }

    #[test]
    fn ra_joined_to_a_consonant_is_left_alone() {
        for text in ["ಕ್ರೃ", "ಸ್ತ್ರೃ", "ಕ್\u{200D}ರೃ", "ಕ್\u{200C}ರೃ", "ರ್\u{200C}ರೃ"]
        {
            assert!(found(text).is_empty(), "{text:?}");
        }
    }

    #[test]
    fn malformed_syllables_are_left_to_the_syllable_check() {
        assert!(found("ರೃಾ").is_empty());
        // A nukta after the sign is a stray mark of its own; the ರೃ before it still counts.
        assert_eq!(found("ರೃ\u{0CBC}"), ["ರೃ"]);
    }

    #[test]
    fn correct_spellings_are_not_typos() {
        for text in [
            "ನೈಋತ್ಯ",
            "ನೈರುತ್ಯ",
            "ಕೃಷ್ಣ",
            "ರ",
            "ರಿ",
            "ರೄ",
            "ಋ",
            "ರ಼ೃ",
            "ನಿಋತಿ",
            "",
            "kannada",
        ] {
            assert!(found(text).is_empty(), "{text:?}");
        }
    }

    #[test]
    fn baraha_and_sanskrit_romanizations_of_r_and_r_vowel_are_flagged() {
        let baraha = transliterate("nair~rutya", Scheme::Baraha, Scheme::Kannada);
        let iast = transliterate("nairṛtya", Scheme::Iast, Scheme::Kannada);
        let iso = transliterate("nairr̥tya", Scheme::Iso15919, Scheme::Kannada);
        assert_eq!(baraha, "ನೈರೃತ್ಯ");
        assert_eq!(iast, "ನೈರೃತ್ಯ");
        assert_eq!(iso, "ನೈರೃತ್ಯ");
        assert_eq!(found(&iast), ["ರೃ"]);
    }

    #[test]
    fn ra_virama_before_vocalic_r_suggests_the_vowel_letter() {
        assert_eq!(fixed("ನೈರ್ಋತ್ಯ"), "ನೈಋತ್ಯ");
        assert_eq!(fixed("ನಿರ್ಋತಿ"), "ನಿಋತಿ");
        assert_eq!(fixed("ರ್ಋಷಿ"), "ಋಷಿ");
        assert_eq!(fixed("ರ್ಋಂ"), "ಋಂ");
        assert_eq!(fixed("ರ್ಋ ರ್ಋ"), "ಋ ಋ");
        assert_eq!(fixed("ನೈರ್ಋತ್ಯ ನೈರೃತ್ಯ"), "ನೈಋತ್ಯ ನೈಋತ್ಯ");
        let rules: Vec<Rule> = find("ನೈರ್ಋತ್ಯ ನೈರೃತ್ಯ").iter().map(|t| t.rule).collect();
        assert_eq!(rules, [Rule::RaViramaVocalicR, Rule::RaVocalicR]);
    }

    #[test]
    fn ra_virama_before_other_vowel_letters_is_left_alone() {
        // Spellings from Kannada Wikipedia: abbreviations, loanwords and sandhi.
        for text in [
            "ಆರ್ಎನ್ಎ",
            "ಕುರ್ಆನ್",
            "ಹೈದರ್ಅಲಿ",
            "ಫಾರ್ಎವರ್",
            "ಪುನರ್ಏಕೀಕರಣ",
            "ಎಂಆರ್ಐ",
            "ಆರ್ಒಸಿ",
            "ಆರ್ಓಕೆ",
            "ಜಫರ್ಉಲ್ಲಾ",
            "ಆರ್ಈಆರ್",
            "ರ್ಔ",
            "ರ್ೠ",
            "ರ್ಌ",
        ] {
            assert!(found(text).is_empty(), "{text:?}");
        }
    }

    #[test]
    fn ra_virama_joined_or_apart_from_vocalic_r_is_left_alone() {
        for text in [
            "ನಿರ್\u{200C}ಋತಿ",
            "ನಿರ್\u{200D}ಋತಿ",
            "ರ್ ಋ",
            "ರ್\nಋ",
            "ಗ್ರ್ಋ",
            "ಕ್\u{200C}ರ್ಋ",
            "ರ಼್ಋ",
            "ರ್ಋಾ",
            "ರ",
            "ರ್",
        ] {
            assert!(found(text).is_empty(), "{text:?}");
        }
    }

    #[test]
    fn nudi_vocalic_r_with_an_arkavattu_gets_a_suggestion() {
        use crate::legacy::{from_unicode, to_unicode, Layout, Options};
        // 49 C4 is ಋ in Nudi; F0 and F2 are its 2 arkavattu glyphs.
        for arka in ['\u{F0}', '\u{F2}'] {
            let typed = format!(
                "{}IÄ{arka}{}",
                from_unicode("ನೈ", Layout::Nudi).text,
                from_unicode("ತ್ಯ", Layout::Nudi).text
            );
            let converted = to_unicode(&typed, Layout::Nudi, &Options::default());
            assert_eq!(converted.text, "ನೈರ್ಋತ್ಯ");
            assert_eq!(fixed(&converted.text), "ನೈಋತ್ಯ");
        }
    }

    #[test]
    fn ranges_are_byte_offsets_in_order() {
        let text = "a ನೈರೃತ್ಯ, ರೃಷಿ";
        let starts: Vec<usize> = find(text).iter().map(|t| t.range.start).collect();
        assert_eq!(starts, [8, 25]);
    }

    #[test]
    fn rule_names_are_stable() {
        assert_eq!(Rule::RaVocalicR.name(), "ra-vocalic-r");
        assert_eq!(Rule::RaViramaVocalicR.name(), "ra-virama-vocalic-r");
    }

    #[test]
    fn suggestions_are_well_formed() {
        let text = fixed("ನೈರೃತ್ಯ ರೃಂ ನೈರ್ಋತ್ಯ ರ್ಋಂ");
        assert!(aksharas(&text).all(|a| a.is_well_formed()));
        assert!(find(&text).is_empty());
    }
}
