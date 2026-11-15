//! Character classes for the Kannada Unicode block (U+0C80 to U+0CFF).
//!
//! Unicode names U+0CDE "KANNADA LETTER FA", but the character is the Old Kannada
//! letter LLLA (ೞ). This crate treats it as LLLA throughout.

/// Virama (halant), U+0CCD.
pub const VIRAMA: char = '\u{0CCD}';
/// Nukta, U+0CBC.
pub const NUKTA: char = '\u{0CBC}';
/// Anusvara, U+0C82.
pub const ANUSVARA: char = '\u{0C82}';
/// Zero width joiner.
pub const ZWJ: char = '\u{200D}';
/// Zero width non-joiner.
pub const ZWNJ: char = '\u{200C}';

/// Old Kannada letter RRA (ಱ).
pub const RRA: char = '\u{0CB1}';
/// Old Kannada letter LLLA (ೞ).
pub const LLLA: char = '\u{0CDE}';
/// Nakaara pollu (ೝ), a standalone form of ನ್.
pub const NAKAARA_POLLU: char = '\u{0CDD}';

/// How a character behaves inside an akshara.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    /// Independent vowel, such as ಅ or ೠ.
    Vowel,
    /// Consonant, such as ಕ, ಱ or ೞ.
    Consonant,
    /// Dependent vowel sign or length mark, such as ಾ or ೕ.
    VowelSign,
    /// Virama.
    Virama,
    /// Nukta.
    Nukta,
    /// Sign that follows a syllable: anusvara, visarga, candrabindu,
    /// jihvamuliya, upadhmaniya, or the combining anusvara above right.
    Modifier,
    /// Kannada digit ೦ to ೯.
    Digit,
    /// Any other character inside the Kannada block, such as avagraha,
    /// nakaara pollu, siddham or the spacing candrabindu.
    OtherKannada,
    /// Zero width joiner.
    Zwj,
    /// Zero width non-joiner.
    Zwnj,
    /// Anything outside the Kannada block.
    Other,
}

/// Classifies a character.
pub fn class(c: char) -> Class {
    match c {
        '\u{0C85}'..='\u{0C8C}' | '\u{0C8E}'..='\u{0C90}' | '\u{0C92}'..='\u{0C94}' => Class::Vowel,
        '\u{0CE0}' | '\u{0CE1}' => Class::Vowel,
        '\u{0C95}'..='\u{0CA8}' | '\u{0CAA}'..='\u{0CB3}' | '\u{0CB5}'..='\u{0CB9}' => {
            Class::Consonant
        }
        LLLA => Class::Consonant,
        '\u{0CBE}'..='\u{0CC4}' | '\u{0CC6}'..='\u{0CC8}' | '\u{0CCA}'..='\u{0CCC}' => {
            Class::VowelSign
        }
        '\u{0CD5}' | '\u{0CD6}' | '\u{0CE2}' | '\u{0CE3}' => Class::VowelSign,
        VIRAMA => Class::Virama,
        NUKTA => Class::Nukta,
        '\u{0C81}'..='\u{0C83}' | '\u{0CF1}'..='\u{0CF3}' => Class::Modifier,
        '\u{0CE6}'..='\u{0CEF}' => Class::Digit,
        ZWJ => Class::Zwj,
        ZWNJ => Class::Zwnj,
        '\u{0C80}'..='\u{0CFF}' => Class::OtherKannada,
        _ => Class::Other,
    }
}

/// True if `c` is in the Kannada block (U+0C80 to U+0CFF).
pub fn is_kannada(c: char) -> bool {
    ('\u{0C80}'..='\u{0CFF}').contains(&c)
}

/// True if `c` is a Kannada consonant, including ಱ and ೞ.
pub fn is_consonant(c: char) -> bool {
    class(c) == Class::Consonant
}

/// True if `c` is a Kannada independent vowel.
pub fn is_vowel(c: char) -> bool {
    class(c) == Class::Vowel
}

/// True if `c` is a Kannada dependent vowel sign or length mark.
pub fn is_vowel_sign(c: char) -> bool {
    class(c) == Class::VowelSign
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_core_letters() {
        assert_eq!(class('ಕ'), Class::Consonant);
        assert_eq!(class('ಅ'), Class::Vowel);
        assert_eq!(class('ಾ'), Class::VowelSign);
        assert_eq!(class('್'), Class::Virama);
        assert_eq!(class('ಂ'), Class::Modifier);
        assert_eq!(class('೭'), Class::Digit);
        assert_eq!(class('a'), Class::Other);
    }

    #[test]
    fn classifies_old_kannada_letters() {
        assert_eq!(class(RRA), Class::Consonant);
        assert_eq!(class(LLLA), Class::Consonant);
        assert_eq!(class('ೠ'), Class::Vowel);
        assert_eq!(class('ೡ'), Class::Vowel);
        assert_eq!(class('ೢ'), Class::VowelSign);
        assert_eq!(class('ೣ'), Class::VowelSign);
        assert_eq!(class('ೄ'), Class::VowelSign);
        assert_eq!(class(NAKAARA_POLLU), Class::OtherKannada);
    }

    #[test]
    fn unassigned_gaps_are_not_letters() {
        for c in [
            '\u{0C8D}', '\u{0C91}', '\u{0CA9}', '\u{0CB4}', '\u{0CC5}', '\u{0CC9}',
        ] {
            assert_eq!(class(c), Class::OtherKannada, "{:04X}", c as u32);
        }
    }
}
