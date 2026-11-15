//! Repairs offered for a malformed akshara. Every repair is checked: its text
//! must segment into well-formed aksharas only.

use std::ops::Range;

use kannadi::chars::{class, Class};
use kannadi::{aksharas, normalize, AksharaKind};

/// How a repair changes the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Unicode normalization makes the syllable valid, such as ಉ + ಾ to ಊ.
    Normalize,
    /// Removes the space or stray glyph that separates a sign from the letter before it.
    Join,
    /// Removes the signs that cannot attach.
    Remove,
    /// Keeps the first of 2 vowel signs.
    KeepFirst,
    /// Keeps the last of 2 vowel signs.
    KeepLast,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Normalize => "normalize",
            Kind::Join => "join",
            Kind::Remove => "remove",
            Kind::KeepFirst => "keep-first",
            Kind::KeepLast => "keep-last",
        }
    }
}

/// A replacement for `range` of the text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fix {
    pub range: Range<usize>,
    pub replacement: String,
    pub kind: Kind,
}

/// Up to 4 repairs for the malformed akshara at `range` (byte offsets), most
/// likely first. Empty if `range` is not exactly 1 malformed akshara.
pub fn fixes(text: &str, range: Range<usize>) -> Vec<Fix> {
    let Some(seg) = text.get(range.clone()) else {
        return Vec::new();
    };
    let mut parts = aksharas(seg);
    let (Some(akshara), None) = (parts.next(), parts.next()) else {
        return Vec::new();
    };
    if akshara.is_well_formed() {
        return Vec::new();
    }

    let mut out: Vec<Fix> = Vec::new();
    let mut offer = |range: Range<usize>, replacement: String, kind: Kind| {
        let valid = aksharas(&replacement).all(|a| a.is_well_formed());
        let changed = text[range.clone()] != replacement;
        if valid
            && changed
            && !out
                .iter()
                .any(|f| f.range == range && f.replacement == replacement)
        {
            out.push(Fix {
                range,
                replacement,
                kind,
            });
        }
    };

    offer(range.clone(), normalize(seg), Kind::Normalize);

    if akshara.kind == AksharaKind::Other {
        if let Some(prev) = previous_consonant(text, range.start) {
            let signs: String = seg.chars().filter(|&c| class(c).is_mark()).collect();
            let joined = format!("{}{signs}", &text[prev.clone()]);
            offer(prev.start..range.end, joined, Kind::Join);
        }
    }

    let kept: String = seg
        .chars()
        .filter(|&c| match akshara.kind {
            AksharaKind::Other => !class(c).is_mark(),
            AksharaKind::Vowel => !matches!(class(c), Class::VowelSign | Class::Nukta),
            AksharaKind::Consonant => true,
        })
        .collect();
    offer(range.clone(), kept, Kind::Remove);

    if akshara.kind == AksharaKind::Consonant {
        let composed = normalize(seg);
        let signs: Vec<usize> = composed
            .char_indices()
            .filter(|&(_, c)| class(c) == Class::VowelSign)
            .map(|(i, _)| i)
            .collect();
        if let (Some(&first), Some(&last)) = (signs.first(), signs.last()) {
            let without = |keep: usize| -> String {
                composed
                    .char_indices()
                    .filter(|&(i, c)| class(c) != Class::VowelSign || i == keep)
                    .map(|(_, c)| c)
                    .collect()
            };
            offer(range.clone(), without(first), Kind::KeepFirst);
            offer(range.clone(), without(last), Kind::KeepLast);
        }
    }

    out.truncate(4);
    out
}

/// The consonant akshara that ends just before the segment starting at `at`,
/// skipping the non-mark characters at the start of that segment.
fn previous_consonant(text: &str, at: usize) -> Option<Range<usize>> {
    let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
    let word_start = text[line_start..at]
        .trim_end()
        .rfind(char::is_whitespace)
        .map_or(line_start, |i| line_start + i + 1);
    let mut pos = word_start;
    let mut last = None;
    for a in aksharas(&text[word_start..at]) {
        last = Some((pos..pos + a.text.len(), a.kind));
        pos += a.text.len();
    }
    match last {
        Some((range, AksharaKind::Consonant)) if range.end == at => Some(range),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn replacements(text: &str, bad: &str) -> Vec<(String, Kind)> {
        let start = text.find(bad).unwrap();
        fixes(text, start..start + bad.len())
            .into_iter()
            .map(|f| {
                let mut fixed = text.to_string();
                fixed.replace_range(f.range, &f.replacement);
                (fixed, f.kind)
            })
            .collect()
    }

    #[test]
    fn joins_a_sign_split_off_by_a_stray_glyph() {
        // From a 2002 RDPR page: an unknown glyph between ಙ and its virama.
        let fixes = replacements("ಪ್ರಙõ್ ಬೆಳೆಸುವ", "õ್");
        assert_eq!(fixes[0], ("ಪ್ರಙ್ ಬೆಳೆಸುವ".to_string(), Kind::Join));
        assert!(fixes.contains(&("ಪ್ರಙõ ಬೆಳೆಸುವ".to_string(), Kind::Remove)));
    }

    #[test]
    fn joins_a_sign_typed_after_a_space() {
        let fixes = replacements("ಕನ್ನಡ ಮನ ೆ", " ೆ");
        assert_eq!(fixes[0], ("ಕನ್ನಡ ಮನೆ".to_string(), Kind::Join));
        assert_eq!(fixes[1], ("ಕನ್ನಡ ಮನ ".to_string(), Kind::Remove));
    }

    #[test]
    fn normalizes_a_vowel_typed_in_2_parts() {
        let fixes = replacements("ಉಾರು", "ಉಾ");
        assert_eq!(fixes[0], ("ಊರು".to_string(), Kind::Normalize));
    }

    #[test]
    fn removes_a_sign_on_a_vowel_letter() {
        let fixes = replacements("ಅೆ", "ಅೆ");
        assert_eq!(fixes, [("ಅ".to_string(), Kind::Remove)]);
    }

    #[test]
    fn keeps_either_of_2_vowel_signs() {
        let fixes = replacements("ಕಾೀ", "ಕಾೀ");
        assert_eq!(
            fixes,
            [
                ("ಕಾ".to_string(), Kind::KeepFirst),
                ("ಕೀ".to_string(), Kind::KeepLast)
            ]
        );
    }

    #[test]
    fn no_fixes_for_well_formed_or_partial_ranges() {
        assert!(fixes("ಕನ್ನಡ", 0..3).is_empty());
        assert!(fixes("ಕಾೀ", 0..1).is_empty());
        assert!(fixes("ಕ ಾಾ", 1..10).is_empty());
    }
}
