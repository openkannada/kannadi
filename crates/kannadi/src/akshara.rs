//! Akshara (orthographic syllable) segmentation.
//!
//! An akshara is the unit a Kannada reader sees as one letter: ಕ, ಕ್ಷೇ, ಸ್ತ್ರೀ, ಅಂ.
//! The rules, written as a pattern over [`Class`] values:
//!
//! ```text
//! vowel akshara      = Vowel (VowelSign | Nukta | Modifier)*
//! consonant akshara  = (Consonant Nukta* Virama ZWJ?)* Consonant Nukta* VowelSign* Modifier*
//!                    | (Consonant Nukta* Virama ZWJ?)* Consonant Nukta* Virama (ZWNJ | ZWJ)?
//! ```
//!
//! A virama followed by ZWNJ always ends the akshara, so ಡ್‌ಗೆ splits as ಡ್‌ + ಗೆ.
//! Characters outside the Kannada block become one segment each, together with
//! any Kannada marks that follow them (for example a dotted circle used to show
//! a vowel sign in a chart). Concatenating the segments always gives back the
//! input exactly.

use crate::chars::{class, Class, NAKAARA_POLLU};

/// What kind of segment an [`Akshara`] is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AksharaKind {
    /// Starts with an independent vowel: ಅ, ಐ, ಅಂ.
    Vowel,
    /// Starts with a consonant: ಕ, ಕ್ಷ, ನ್.
    Consonant,
    /// Anything else: digits, punctuation, spaces, Latin text, stray marks.
    Other,
}

/// One segment of text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Akshara<'a> {
    /// The text of the segment, borrowed from the input.
    pub text: &'a str,
    /// The segment kind.
    pub kind: AksharaKind,
}

impl Akshara<'_> {
    /// True for vowel and consonant aksharas.
    pub fn is_syllable(&self) -> bool {
        self.kind != AksharaKind::Other
    }
}

/// Iterator over the aksharas of a string. Created by [`aksharas`].
#[derive(Clone, Debug)]
pub struct Aksharas<'a> {
    text: &'a str,
    pos: usize,
}

/// Splits `text` into aksharas and other segments.
///
/// ```
/// let parts: Vec<&str> = kannadi::aksharas("ಕನ್ನಡ ಸ್ತ್ರೀ").map(|a| a.text).collect();
/// assert_eq!(parts, ["ಕ", "ನ್ನ", "ಡ", " ", "ಸ್ತ್ರೀ"]);
/// ```
pub fn aksharas(text: &str) -> Aksharas<'_> {
    Aksharas { text, pos: 0 }
}

/// Splits `text` into segment strings. Shorthand for collecting [`aksharas`].
pub fn split(text: &str) -> Vec<&str> {
    aksharas(text).map(|a| a.text).collect()
}

/// Counts vowel and consonant aksharas, ignoring spaces, digits and other text.
///
/// ```
/// assert_eq!(kannadi::count_aksharas("ಕನ್ನಡ ನಾಡು, 2026"), 5);
/// ```
pub fn count_aksharas(text: &str) -> usize {
    aksharas(text).filter(Akshara::is_syllable).count()
}

struct Cursor<'a> {
    rest: std::iter::Peekable<std::str::CharIndices<'a>>,
}

impl Cursor<'_> {
    fn peek_class(&mut self) -> Option<Class> {
        self.rest.peek().map(|&(_, c)| class(c))
    }

    fn bump(&mut self) {
        self.rest.next();
    }

    fn eat_while(&mut self, pred: impl Fn(Class) -> bool) {
        while self.peek_class().is_some_and(&pred) {
            self.bump();
        }
    }

    fn peek_second_class(&self) -> Option<Class> {
        let mut it = self.rest.clone();
        it.next();
        it.next().map(|(_, c)| class(c))
    }
}

fn is_mark(c: Class) -> bool {
    matches!(
        c,
        Class::VowelSign | Class::Virama | Class::Nukta | Class::Modifier
    )
}

impl<'a> Iterator for Aksharas<'a> {
    type Item = Akshara<'a>;

    fn next(&mut self) -> Option<Akshara<'a>> {
        let rest = &self.text[self.pos..];
        let mut cur = Cursor {
            rest: rest.char_indices().peekable(),
        };
        let (_, first) = *cur.rest.peek()?;
        cur.bump();
        let kind = match class(first) {
            Class::Vowel => {
                cur.eat_while(|c| matches!(c, Class::VowelSign | Class::Nukta | Class::Modifier));
                AksharaKind::Vowel
            }
            Class::Consonant => {
                consonant_cluster(&mut cur);
                AksharaKind::Consonant
            }
            _ if first == NAKAARA_POLLU => {
                cur.eat_while(|c| c == Class::Modifier);
                AksharaKind::Consonant
            }
            _ => {
                cur.eat_while(is_mark);
                AksharaKind::Other
            }
        };
        let len = cur.rest.peek().map_or(rest.len(), |&(i, _)| i);
        self.pos += len;
        Some(Akshara {
            text: &rest[..len],
            kind,
        })
    }
}

/// Consumes the remainder of a consonant akshara whose first consonant was already read.
fn consonant_cluster(cur: &mut Cursor<'_>) {
    loop {
        cur.eat_while(|c| c == Class::Nukta);
        if cur.peek_class() != Some(Class::Virama) {
            cur.eat_while(|c| c == Class::VowelSign);
            cur.eat_while(|c| c == Class::Modifier);
            return;
        }
        cur.bump();
        match cur.peek_class() {
            Some(Class::Consonant) => cur.bump(),
            Some(Class::Zwj) if cur.peek_second_class() == Some(Class::Consonant) => {
                cur.bump();
                cur.bump();
            }
            Some(Class::Zwj | Class::Zwnj) => {
                cur.bump();
                return;
            }
            _ => return,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_simple_word() {
        assert_eq!(split("ಕನ್ನಡ"), ["ಕ", "ನ್ನ", "ಡ"]);
    }

    #[test]
    fn keeps_long_conjunct_with_vowel_sign_and_anusvara() {
        assert_eq!(split("ಸ್ತ್ರೀಯರಂ"), ["ಸ್ತ್ರೀ", "ಯ", "ರಂ"]);
        assert_eq!(split("ಕ್ಷೇತ್ರ"), ["ಕ್ಷೇ", "ತ್ರ"]);
    }

    #[test]
    fn vowel_akshara_takes_modifiers() {
        assert_eq!(split("ಅಂಗ"), ["ಅಂ", "ಗ"]);
        assert_eq!(split("ಓಃ"), ["ಓಃ"]);
    }

    #[test]
    fn dead_consonant_at_word_end() {
        assert_eq!(split("ಬಾೞ್ ಮಾಡ್"), ["ಬಾ", "ೞ್", " ", "ಮಾ", "ಡ್"]);
    }

    #[test]
    fn zwnj_after_virama_ends_akshara() {
        assert_eq!(split("ಡ್\u{200C}ಗೆ"), ["ಡ್\u{200C}", "ಗೆ"]);
    }

    #[test]
    fn zwj_after_virama_keeps_cluster() {
        assert_eq!(split("ರ್\u{200D}ಕ"), ["ರ್\u{200D}ಕ"]);
    }

    #[test]
    fn nukta_stays_with_consonant() {
        assert_eq!(split("ಫ಼ೈಲ್"), ["ಫ಼ೈ", "ಲ್"]);
    }

    #[test]
    fn decomposed_vowel_signs_stay_together() {
        assert_eq!(split("ಕೆ\u{0CD5}ಳು"), ["ಕೆ\u{0CD5}", "ಳು"]);
    }

    #[test]
    fn stray_marks_and_other_text() {
        assert_eq!(
            split("\u{25CC}ಾ ab ೧೨"),
            ["\u{25CC}ಾ", " ", "a", "b", " ", "೧", "೨"]
        );
        assert_eq!(split("್ಕ"), ["್", "ಕ"]);
    }

    #[test]
    fn nakaara_pollu_is_a_consonant_akshara() {
        assert_eq!(split("ಅವೝ"), ["ಅ", "ವ", "ೝ"]);
        assert_eq!(count_aksharas("ಅವೝ"), 3);
        assert_eq!(split("ೝಂ\u{0951}"), ["ೝಂ\u{0951}"]);
    }

    #[test]
    fn vedic_accents_stay_with_their_syllable() {
        assert_eq!(split("ಅ\u{0951}ಗ್ನಿ"), ["ಅ\u{0951}", "ಗ್ನಿ"]);
    }

    #[test]
    fn kinds_and_counts() {
        let kinds: Vec<_> = aksharas("ಅಕ1").map(|a| a.kind).collect();
        assert_eq!(
            kinds,
            [
                AksharaKind::Vowel,
                AksharaKind::Consonant,
                AksharaKind::Other
            ]
        );
        assert_eq!(count_aksharas(""), 0);
        assert_eq!(count_aksharas("ಸ್ತ್ರೀಯರು"), 3);
    }
}
