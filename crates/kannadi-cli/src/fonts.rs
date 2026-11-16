//! Font names: which ones are legacy fonts, and in which glyph layout.

use kannadi::legacy::Layout;

/// The glyph layout of text set in `font`, or `None` for a font that is not a
/// legacy font.
///
/// - [`Layout::Nudi`]: the ANSI Nudi fonts up to version 5 (Nudi 01 e, Nudi
///   Akshara, Nudi B-Akshara) and Baraha's BRH fonts for Kannada. The Unicode
///   Nudi fonts (Nudi Parijatha and others) are not legacy fonts.
/// - [`Layout::Akruti`]: AkrutiKndPadma and its weights. Other Akruti Kannada
///   fonts, such as AkrutiKndPadmini, were not checked and are left out.
///
/// `extra` adds exact names, ignoring case, each with its layout.
pub fn layout(font: &str, extra: &[(String, Layout)]) -> Option<Layout> {
    let font = font.trim().trim_matches(|c| c == '"' || c == '\'');
    if let Some((_, layout)) = extra.iter().find(|(e, _)| e.eq_ignore_ascii_case(font)) {
        return Some(*layout);
    }
    let key = key(font);
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

/// True if `font` is a legacy font in any layout.
pub fn is_legacy(font: &str, extra: &[(String, Layout)]) -> bool {
    layout(font, extra).is_some()
}

/// True for the `k` variants of the numbered Nudi fonts, such as Nudi 04 k, which
/// draw Kannada digits at the ASCII digit codes.
pub fn has_kannada_digits(font: &str) -> bool {
    key(font).strip_prefix("nudi").is_some_and(|rest| {
        let b = rest.as_bytes();
        b.len() >= 3 && b[0].is_ascii_digit() && b[1].is_ascii_digit() && b[2] == b'k'
    })
}

/// A font name in lowercase letters and digits only.
fn key(font: &str) -> String {
    font.chars()
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

/// The first family in a CSS `font-family` list, without quotes.
pub fn first_family(list: &str) -> &str {
    list.split(',')
        .next()
        .unwrap_or_default()
        .trim()
        .trim_matches(|c| c == '"' || c == '\'')
}

/// A `font-family` list with each legacy family replaced by `font`, in `quote`
/// marks when given.
pub fn replace_legacy(
    list: &str,
    font: &str,
    quote: Option<char>,
    extra: &[(String, Layout)],
) -> String {
    list.split(',')
        .map(|family| {
            if is_legacy(family, extra) {
                let lead = &family[..family.len() - family.trim_start().len()];
                let q = quote.map(String::from).unwrap_or_default();
                format!("{lead}{q}{font}{q}")
            } else {
                family.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_font_names() {
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
            assert_eq!(layout(f, &[]), Some(Layout::Nudi), "{f}");
        }
        for f in [
            "AkrutiKndPadma",
            "AkrutiKndPadmaDemi",
            "AkrutiKndPadma,Bold",
            "Akruti Knd Padma Demib",
        ] {
            assert_eq!(layout(f, &[]), Some(Layout::Akruti), "{f}");
        }
        for f in [
            "Nudi Parijatha",
            "NudiUni01e",
            "Tunga",
            "BRH Devanagari",
            "Noto Sans Kannada",
            "Arial",
            "AkrutiKndPadmini",
            "AkrutiDevPriya",
        ] {
            assert!(!is_legacy(f, &[]), "{f}");
        }
        let extra = [("my font".to_string(), Layout::Akruti)];
        assert_eq!(layout("My Font", &extra), Some(Layout::Akruti));
    }

    #[test]
    fn kannada_digit_fonts() {
        for f in ["Nudi 04 k", "Nudi01k", "Nudi01k-Bold", "Nudi 05 k Bold"] {
            assert!(has_kannada_digits(f), "{f}");
        }
        for f in ["Nudi 01 e", "NudiAkshara", "BRH Kannada"] {
            assert!(!has_kannada_digits(f), "{f}");
        }
    }

    #[test]
    fn family_lists() {
        assert_eq!(first_family(" \"BRH Kannada\", serif"), "BRH Kannada");
        assert_eq!(
            replace_legacy(
                "\"BRH Kannada\", serif",
                "Noto Sans Kannada",
                Some('"'),
                &[]
            ),
            "\"Noto Sans Kannada\", serif"
        );
        assert_eq!(replace_legacy("BRH Kannada", "Tunga", None, &[]), "Tunga");
        assert_eq!(replace_legacy("Arial", "X", Some('"'), &[]), "Arial");
    }
}
