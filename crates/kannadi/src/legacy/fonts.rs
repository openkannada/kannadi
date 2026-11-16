//! Legacy font names: the layout a font draws, and which fonts draw Kannada digits.

use super::Layout;

/// The glyph layout of text set in the font named `name`, or `None` for a font
/// this crate has no table for. Case, spaces, quotes and punctuation in the name
/// are ignored.
///
/// - [`Layout::Nudi`]: the ANSI Nudi fonts up to version 5 (Nudi 01 e, Nudi
///   Akshara, Nudi B-Akshara) and Baraha's BRH fonts for Kannada. The Unicode
///   Nudi fonts, such as Nudi Parijatha, and Baraha's BRH fonts for other scripts
///   are not legacy Kannada fonts.
/// - [`Layout::Akruti`]: AkrutiKndPadma and its weights. Other Akruti Kannada
///   fonts, such as AkrutiKndPadmini, were not checked and are left out.
///
/// ```
/// use kannadi::legacy::{font_layout, Layout};
///
/// assert_eq!(font_layout("BRH Kannada"), Some(Layout::Nudi));
/// assert_eq!(font_layout("AkrutiKndPadmaDemi"), Some(Layout::Akruti));
/// assert_eq!(font_layout("Nudi Parijatha"), None);
/// ```
pub fn font_layout(name: &str) -> Option<Layout> {
    let key = key(name);
    if let Some(rest) = key.strip_prefix("brh") {
        return (!BRH_OTHER_SCRIPTS.iter().any(|s| rest.starts_with(s))).then_some(Layout::Nudi);
    }
    if key.starts_with("akrutikndpadma") {
        return Some(Layout::Akruti);
    }
    let rest = key.strip_prefix("nudi")?;
    let numbered = rest.len() >= 3 && rest[..2].bytes().all(|b| b.is_ascii_digit());
    (numbered || rest.starts_with("akshar") || rest.starts_with("bakshar")).then_some(Layout::Nudi)
}

/// True for a font that stores Kannada as glyph codes in place of letters: the
/// fonts [`font_layout`] knows, and Shree-Lipi's Shree-Kan fonts, such as
/// Shree-Kan-0850, which have no table yet. Text in a legacy font with no table
/// cannot be converted, and should not be taken for Latin text either.
///
/// ```
/// use kannadi::legacy::{font_layout, is_legacy_font};
///
/// assert!(is_legacy_font("Shree-Kan-0850"));
/// assert_eq!(font_layout("Shree-Kan-0850"), None);
/// assert!(!is_legacy_font("Arial"));
/// ```
pub fn is_legacy_font(name: &str) -> bool {
    font_layout(name).is_some() || is_shree_lipi(&key(name))
}

/// True for the `k` variants of the numbered Nudi fonts, such as Nudi 04 k, which
/// draw Kannada digits at the ASCII digit codes.
///
/// ```
/// use kannadi::legacy::draws_kannada_digits;
///
/// assert!(draws_kannada_digits("Nudi 04 k"));
/// assert!(!draws_kannada_digits("Nudi 01 e"));
/// ```
pub fn draws_kannada_digits(name: &str) -> bool {
    key(name).strip_prefix("nudi").is_some_and(|rest| {
        let b = rest.as_bytes();
        b.len() >= 3 && b[0].is_ascii_digit() && b[1].is_ascii_digit() && b[2] == b'k'
    })
}

/// Shree-Kan followed by its number, as in Shree-Kan-0850 and SHREE-KAN-0851.
fn is_shree_lipi(key: &str) -> bool {
    key.strip_prefix("shreekan")
        .is_some_and(|rest| rest.bytes().next().is_some_and(|b| b.is_ascii_digit()))
}

/// A font name in lowercase letters and digits only.
fn key(name: &str) -> String {
    name.chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase()
}

/// Baraha also made BRH fonts for these scripts, in other layouts.
const BRH_OTHER_SCRIPTS: [&str; 10] = [
    "devanagari",
    "sanskrit",
    "hindi",
    "marathi",
    "tamil",
    "telugu",
    "malayalam",
    "bengali",
    "gujarati",
    "gurmukhi",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nudi_and_brh_fonts() {
        for f in [
            "Nudi 01 e",
            "Nudi 05 k",
            "NudiAkshara",
            "Nudi Akshar-14",
            "Nudi B-Akshara",
            "BRH Kannada",
            "BRHKannada-Bold",
            "\"BRH Sirigannada\"",
            "brh kasturi",
        ] {
            assert_eq!(font_layout(f), Some(Layout::Nudi), "{f}");
        }
    }

    #[test]
    fn akruti_fonts() {
        for f in [
            "AkrutiKndPadma",
            "AkrutiKndPadmaDemi",
            "AkrutiKndPadma,Bold",
            "Akruti Knd Padma Demib",
        ] {
            assert_eq!(font_layout(f), Some(Layout::Akruti), "{f}");
        }
    }

    #[test]
    fn unicode_and_other_script_fonts_are_not_legacy() {
        for f in [
            "Nudi Parijatha",
            "NudiUni01e",
            "Nudi Unicode 01",
            "BRH Devanagari",
            "BRH Telugu",
            "Tunga",
            "Noto Sans Kannada",
            "Arial",
            "AkrutiKndPadmini",
            "AkrutiDevPriya",
            "Shree Kannada",
        ] {
            assert_eq!(font_layout(f), None, "{f}");
            assert!(!is_legacy_font(f), "{f}");
        }
    }

    #[test]
    fn shree_lipi_fonts_are_legacy_with_no_table() {
        for f in ["Shree-Kan-0850", "SHREE-KAN-0851", "Shree Kan 0853 Bold"] {
            assert!(is_legacy_font(f), "{f}");
            assert_eq!(font_layout(f), None, "{f}");
        }
        assert!(is_legacy_font("BRH Kannada"));
    }

    #[test]
    fn kannada_digit_fonts() {
        for f in ["Nudi 04 k", "Nudi01k", "Nudi01k-Bold", "Nudi 05 k Bold"] {
            assert!(draws_kannada_digits(f), "{f}");
        }
        for f in ["Nudi 01 e", "NudiAkshara", "BRH Kannada", "AkrutiKndPadma"] {
            assert!(!draws_kannada_digits(f), "{f}");
        }
    }
}
