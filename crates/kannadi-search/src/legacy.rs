//! Legacy font text: lines typed in Nudi, Baraha, Akruti and other legacy fonts, as
//! text extraction from old PDFs and Word files gives them (PÀ£ÀßqÀ for ಕನ್ನಡ).
//!
//! [`detect`] decides whether a line holds legacy text and in which layout, and
//! [`convert_words`] turns it into Unicode word by word, so each word keeps its place
//! in the original line.

use std::ops::Range;

use kannadi::legacy::{best_layout, is_unicode, to_unicode, Layout, Options};

/// The layout of the legacy text in `line`, or `None` when the line has none.
///
/// The words with glyph codes (Windows-1252 characters above ASCII other than quotes
/// and dashes, which legacy layouts pass through) must have at least 3 codes, at least
/// 40% of their characters must be codes, and they must convert cleanly in the best
/// layout: at most 1 word in 10 with an unknown glyph or a malformed syllable. Real
/// Nudi words are 59% codes or more and convert cleanly; accented European words, such
/// as Ñúñez or Größe, fail the conversion.
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
    let coded: Vec<&str> = words(line)
        .map(|r| &line[r])
        .filter(|w| w.chars().any(is_glyph_code))
        .collect();
    let (mut codes, mut chars) = (0, 0);
    for c in coded.iter().flat_map(|w| w.chars()) {
        chars += 1;
        codes += usize::from(is_glyph_code(c));
    }
    if codes < MIN_CODES || codes * 10 < chars * 4 {
        return None;
    }
    let text = coded.join(" ");
    let layout = best_layout(&text);
    let conversion = to_unicode(&text, layout, &options());
    let problems = conversion.unknown.len() + conversion.malformed.len();
    let clean = problems * 10 <= coded.len();
    let kannada = conversion
        .text
        .chars()
        .filter(|&c| kannadi::chars::is_kannada(c))
        .count();
    (clean && kannada >= 2 && !is_unicode(&text, layout)).then_some(layout)
}

/// Glyph codes a line needs: 2 accented letters, as in a name like Eóá, are not enough.
const MIN_CODES: usize = 3;

fn options() -> Options {
    Options {
        zero_as_anusvara: true,
        ..Options::default()
    }
}

/// Windows-1252 characters above ASCII where legacy layouts draw their glyphs. Quotes,
/// dashes and other punctuation in U+2010 to U+2027 pass through a legacy run as
/// themselves, as kannadi's converter treats them.
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
                | '‰'
                | '‹'
                | '›'
                | '€'
                | '™'
        )
}

/// The Unicode text of the words of a legacy `line`, with each word's byte range in
/// `line`. When most words have glyph codes, the line is legacy text throughout and
/// every word converts, as some legacy words use ASCII codes only. Otherwise only the
/// words with glyph codes convert, and the others, such as English words in a
/// Kannada order, stay as they are. Spaces carry no glyph in any layout, so converting
/// word by word gives the same text as converting the line, apart from a zero typed
/// for ಂ at the start of a word.
pub fn convert_words(line: &str, layout: Layout) -> Vec<(Range<usize>, String)> {
    let ranges: Vec<Range<usize>> = words(line).collect();
    let coded = ranges
        .iter()
        .filter(|r| line[(*r).clone()].chars().any(is_glyph_code))
        .count();
    let all = coded * 10 >= ranges.len() * 6;
    ranges
        .into_iter()
        .map(|range| {
            let word = &line[range.clone()];
            let text = if all || word.chars().any(is_glyph_code) {
                to_unicode(word, layout, &options()).text
            } else {
                word.to_string()
            };
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
    fn mixed_lines_convert_only_their_legacy_words() {
        let line = "PÀ£ÀßqÀ ¨sÁµÉ Karnataka Government Order No. 12";
        let layout = detect(line).expect("legacy words");
        let words: Vec<String> = convert_words(line, layout)
            .into_iter()
            .map(|w| w.1)
            .collect();
        assert_eq!(words[..3], ["ಕನ್ನಡ", "ಭಾಷೆ", "Karnataka"]);
    }

    #[test]
    fn other_lines_are_left() {
        for line in [
            "“Hi”",
            "José Ñúñez",
            "±5°C",
            "Größe",
            "¿Qué?",
            "— “No.”",
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
        assert_eq!(detect("eóá"), None);
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
