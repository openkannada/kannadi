//! Font names: the names added on the command line, and `font-family` lists.
//! The rules for known font names are in [`kannadi::legacy::font_layout`].

use kannadi::legacy::Layout;

/// The glyph layout of text set in `font`, or `None` for a font with no table:
/// a name in `extra` (exact, ignoring case) with its layout, or else the layout
/// [`kannadi::legacy::font_layout`] gives.
pub fn layout(font: &str, extra: &[(String, Layout)]) -> Option<Layout> {
    let font = font.trim().trim_matches(|c| c == '"' || c == '\'');
    match extra.iter().find(|(e, _)| e.eq_ignore_ascii_case(font)) {
        Some((_, layout)) => Some(*layout),
        None => kannadi::legacy::font_layout(font),
    }
}

/// True if `font` is a legacy font this tool converts, in any layout.
pub fn is_legacy(font: &str, extra: &[(String, Layout)]) -> bool {
    layout(font, extra).is_some()
}

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
    fn added_names_come_first_and_known_names_follow_the_crate() {
        let extra = [("my font".to_string(), Layout::Akruti)];
        assert_eq!(layout("\"My Font\"", &extra), Some(Layout::Akruti));
        assert_eq!(layout("BRH Kannada", &extra), Some(Layout::Nudi));
        assert_eq!(layout("Shree-Kan-0850", &[]), Some(Layout::ShreeLipi));
        assert_eq!(layout("Shree-Kan-0851", &[]), None);
        assert!(!is_legacy("Arial", &extra));
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
