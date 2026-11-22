//! Legacy font text: lines typed in Nudi, Baraha, Akruti and other legacy fonts, as
//! text extraction from old PDFs and Word files gives them (PÀ£ÀßqÀ for ಕನ್ನಡ).
//!
//! [`detect`] decides whether a line is legacy text and in which layout, and
//! [`convert_words`] turns it into Unicode word by word, so each word keeps its place
//! in the original line.

use std::ops::Range;

use kannadi::legacy::{best_layout, is_unicode, to_unicode, Layout, Options};

/// The layout `line` is typed in, or `None` when the line is Unicode or plain text.
///
/// A line counts as legacy text when at least 2 of its characters, and at least 40%
/// of its characters other than spaces, are Windows-1252 characters above ASCII,
/// which legacy layouts use as glyph codes, and it has fewer Unicode Kannada
/// characters than such codes. Real Nudi lines have 59% or more; accented European
/// text stays near 33% (kannadi's editor page uses the same test).
///
/// ```
/// use kannadi::legacy::Layout;
/// use kannadi_search::legacy::detect;
///
/// assert_eq!(detect("PÀ£ÀßqÀ ¨sÁµÉ"), Some(Layout::Nudi));
/// assert_eq!(detect("ಕನ್ನಡ ಭಾಷೆ"), None);
/// assert_eq!(detect("Café au lait"), None);
/// ```
pub fn detect(line: &str) -> Option<Layout> {
    let mut codes = 0;
    let mut non_space = 0;
    for c in line.chars().filter(|c| !c.is_whitespace()) {
        non_space += 1;
        codes += usize::from(is_glyph_code(c));
    }
    let legacy = codes >= 2 && codes * 10 >= non_space * 4;
    if !legacy {
        return None;
    }
    let layout = best_layout(line);
    (!is_unicode(line, layout)).then_some(layout)
}

/// Windows-1252 characters above ASCII: where legacy layouts draw their glyphs.
fn is_glyph_code(c: char) -> bool {
    ('\u{A1}'..='\u{FF}').contains(&c)
        || matches!(
            c,
            'Œ' | 'œ'
                | 'Š'
                | 'š'
                | 'Ÿ'
                | 'Ž'
                | 'ž'
                | 'ƒ'
                | 'ˆ'
                | '˜'
                | '–'
                | '—'
                | '‘'..='„'
                | '†'..='•'
                | '…'
                | '‰'
                | '‹'
                | '›'
                | '€'
                | '™'
        )
}

/// The Unicode text of each whitespace-separated word of a legacy `line`, with the
/// word's byte range in `line`. Spaces carry no glyph in any layout, so converting
/// word by word gives the same text as converting the line, apart from a zero typed
/// for ಂ at the start of a word.
pub fn convert_words(line: &str, layout: Layout) -> Vec<(Range<usize>, String)> {
    let options = Options {
        zero_as_anusvara: true,
        ..Options::default()
    };
    words(line)
        .map(|range| {
            let text = to_unicode(&line[range.clone()], layout, &options).text;
            (range, text)
        })
        .collect()
}

fn words(line: &str) -> impl Iterator<Item = Range<usize>> + '_ {
    let mut start = None;
    line.char_indices()
        .chain([(line.len(), ' ')])
        .filter_map(move |(i, c)| match (c.is_whitespace(), start) {
            (false, None) => {
                start = Some(i);
                None
            }
            (true, Some(s)) => {
                start = None;
                Some(s..i)
            }
            _ => None,
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_lines_are_found() {
        assert_eq!(detect("PÀ£ÀßqÀ"), Some(Layout::Nudi));
        assert_eq!(detect("¸ÀPÁðgÀzÀ DzÉÃ±À"), Some(Layout::Nudi));
    }

    #[test]
    fn other_lines_are_left() {
        for line in [
            "",
            "ಕನ್ನಡ",
            "Bengaluru 2026",
            "Crème brûlée à la française",
            "ಮಳೆ 30°C ಇದೆ",
            "© 2026",
            "ಕನ್ನಡ ನಾಡು “ಹೊಸ” – 2026",
        ] {
            assert_eq!(detect(line), None, "{line}");
        }
    }

    #[test]
    fn a_single_code_is_not_enough() {
        assert_eq!(detect("À"), None);
        assert_eq!(detect("Àa bcdef"), None);
    }

    #[test]
    fn words_keep_their_place() {
        let line = "PÀ£ÀßqÀ  ¨sÁµÉ";
        let words = convert_words(line, Layout::Nudi);
        assert_eq!(words.len(), 2);
        assert_eq!(&line[words[0].0.clone()], "PÀ£ÀßqÀ");
        assert_eq!(words[0].1, "ಕನ್ನಡ");
        assert_eq!(&line[words[1].0.clone()], "¨sÁµÉ");
        assert_eq!(words[1].1, "ಭಾಷೆ");
    }
}
