//! Spellings that are well formed but are likely typing mistakes.

use std::ops::Range;

use crate::akshara::{aksharas, AksharaKind};

/// Which rule found a [`Typo`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Rule {
    /// ರ with the vowel sign ೃ, written where the vowel letter ಋ is meant:
    /// ನೈರೃತ್ಯ for ನೈಋತ್ಯ. Baraha phonetic typing produces it from `r~r`.
    RaVocalicR,
}

impl Rule {
    /// A stable kebab-case name, such as `ra-vocalic-r`.
    pub const fn name(self) -> &'static str {
        match self {
            Rule::RaVocalicR => "ra-vocalic-r",
        }
    }
}

/// A likely typing mistake: replace `range` (byte offsets) with `replacement`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Typo {
    /// The text to replace.
    pub range: Range<usize>,
    /// The likely intended spelling.
    pub replacement: &'static str,
    /// The rule that found it.
    pub rule: Rule,
}

const RA_VOCALIC_R: &str = "\u{0CB0}\u{0CC3}";

/// Finds likely typing mistakes in `text`, in text order.
///
/// Every spelling found is valid Unicode, so [`crate::Akshara::is_well_formed`]
/// accepts it; these are words that are almost always typed by mistake.
///
/// - ರೃ at the start of an akshara becomes ಋ. Kannada writes the vowel letter
///   in words such as ನೈಋತ್ಯ and ನಿಋತಿ. A subscript ra (ಕ್ರೃ) is left alone, as
///   the intended spelling is not clear.
///
/// ```
/// use kannadi::typos::{typos, Rule};
///
/// let text = "ನೈರೃತ್ಯ";
/// let found = typos(text);
/// assert_eq!(found.len(), 1);
/// assert_eq!(found[0].rule, Rule::RaVocalicR);
/// assert_eq!(&text[found[0].range.clone()], "ರೃ");
/// assert_eq!(found[0].replacement, "ಋ");
/// ```
pub fn typos(text: &str) -> Vec<Typo> {
    let mut out = Vec::new();
    let mut pos = 0;
    for a in aksharas(text) {
        if a.kind == AksharaKind::Consonant && a.text.starts_with(RA_VOCALIC_R) {
            out.push(Typo {
                range: pos..pos + RA_VOCALIC_R.len(),
                replacement: "\u{0C8B}",
                rule: Rule::RaVocalicR,
            });
        }
        pos += a.text.len();
    }
    out
}

/// Replaces every likely typing mistake [`typos`] finds.
///
/// ```
/// assert_eq!(kannadi::typos::fix_typos("ನೈರೃತ್ಯ ದಿಕ್ಕು"), "ನೈಋತ್ಯ ದಿಕ್ಕು");
/// ```
pub fn fix_typos(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut from = 0;
    for typo in typos(text) {
        out.push_str(&text[from..typo.range.start]);
        out.push_str(typo.replacement);
        from = typo.range.end;
    }
    out.push_str(&text[from..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::akshara::aksharas;

    fn found(text: &str) -> Vec<&str> {
        typos(text).iter().map(|t| &text[t.range.clone()]).collect()
    }

    #[test]
    fn ra_with_vocalic_r_sign_becomes_the_vowel_letter() {
        assert_eq!(fix_typos("ನೈರೃತ್ಯ"), "ನೈಋತ್ಯ");
        assert_eq!(fix_typos("ನಿರೃತಿ"), "ನಿಋತಿ");
        assert_eq!(fix_typos("ರೃಷಿ"), "ಋಷಿ");
        assert_eq!(fix_typos("ರೃಂ"), "ಋಂ");
        assert_eq!(fix_typos("ರೃ ರೃ"), "ಋ ಋ");
    }

    #[test]
    fn subscript_ra_is_left_alone() {
        assert!(found("ಕ್ರೃ").is_empty());
        assert!(found("ಸ್ತ್ರೃ").is_empty());
    }

    #[test]
    fn correct_spellings_are_not_typos() {
        for text in ["ನೈಋತ್ಯ", "ನೈರುತ್ಯ", "ಕೃಷ್ಣ", "ರ", "ರಿ", "ರೄ", "ಋ", "", "kannada"]
        {
            assert!(found(text).is_empty(), "{text:?}");
        }
    }

    #[test]
    fn ranges_are_byte_offsets_in_order() {
        let text = "a ನೈರೃತ್ಯ, ರೃಷಿ";
        assert_eq!(found(text), ["ರೃ", "ರೃ"]);
        let starts: Vec<usize> = typos(text).iter().map(|t| t.range.start).collect();
        assert!(starts.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn fixes_are_well_formed() {
        let fixed = fix_typos("ನೈರೃತ್ಯ ರೃಂ");
        assert!(aksharas(&fixed).all(|a| a.is_well_formed()));
        assert!(typos(&fixed).is_empty());
    }
}
