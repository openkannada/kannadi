//! Font names: which ones use the Nudi and Baraha ANSI layout.

/// True if text set in `font` uses the Nudi and Baraha ANSI glyph layout: the
/// ANSI Nudi fonts up to version 5 (Nudi 01 e, Nudi Akshara, Nudi B-Akshara) and
/// Baraha's BRH fonts for Kannada. The Unicode Nudi fonts (Nudi Parijatha and
/// others) are not legacy fonts. `extra` adds exact names, ignoring case.
pub fn is_legacy(font: &str, extra: &[String]) -> bool {
    let font = font.trim().trim_matches(|c| c == '"' || c == '\'');
    if extra.iter().any(|e| e.eq_ignore_ascii_case(font)) {
        return true;
    }
    let key: String = font
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    if let Some(rest) = key.strip_prefix("brh") {
        return !BRH_OTHER_SCRIPTS.iter().any(|s| rest.starts_with(s));
    }
    let Some(rest) = key.strip_prefix("nudi") else {
        return false;
    };
    let numbered = rest.len() >= 3 && rest[..2].bytes().all(|b| b.is_ascii_digit());
    numbered || rest.starts_with("akshara") || rest.starts_with("bakshara")
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
pub fn replace_legacy(list: &str, font: &str, quote: Option<char>, extra: &[String]) -> String {
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
            "Nudi B-Akshara",
            "BRH Kannada",
            "BRHKannada-Bold",
            "\"BRH Sirigannada\"",
            "brh kasturi",
        ] {
            assert!(is_legacy(f, &[]), "{f}");
        }
        for f in [
            "Nudi Parijatha",
            "NudiUni01e",
            "Tunga",
            "BRH Devanagari",
            "Noto Sans Kannada",
            "Arial",
        ] {
            assert!(!is_legacy(f, &[]), "{f}");
        }
        assert!(is_legacy("My Font", &["my font".to_string()]));
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
