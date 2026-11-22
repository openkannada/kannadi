//! Search analysis for Kannada text: a word is found in all its spellings and in its
//! case and plural forms.
//!
//! [`Analyzer::analyze`] turns text into search tokens in 4 steps:
//!
//! 1. **Legacy text.** A line typed in a legacy font, such as PÀ£ÀßqÀ from an old
//!    government PDF, is converted to Unicode with kannadi's converter ([`legacy`]).
//! 2. **Words.** Text splits at Unicode word boundaries (UAX #29), which never fall
//!    inside an akshara: vowel signs, the virama, the nukta and joiners stay with
//!    their letter. A word that mixes digits and Kannada, as in 2026ರಲ್ಲಿ, gives the
//!    number alone.
//! 3. **Spelling.** Each word becomes its search key ([`fold`]): joiners, Old Kannada
//!    letters, the nukta, ೦ typed for ಂ, a nasal with virama for ಂ, Kannada digits
//!    and letter case fold away.
//! 4. **Endings.** Case and plural endings are removed ([`stem`]), so ಮರಗಳಲ್ಲಿ finds ಮರ.
//!
//! ```
//! use kannadi_search::Analyzer;
//!
//! let analyzer = Analyzer::new();
//! let keys = |text| analyzer.keys(text);
//! assert_eq!(keys("ಮರಗಳಲ್ಲಿ ಹಕ್ಕಿ"), ["ಮರ", "ಹಕ್ಕಿ"]);
//! assert_eq!(keys("ಕನ್\u{200C}ನಡದ"), keys("ಕನ್ನಡ"));
//! assert_eq!(keys("PÀ£ÀßqÀ"), ["ಕನ್ನಡ"]);
//! assert_eq!(keys("೨೦೨೬ರಲ್ಲಿ Bengaluru"), ["2026", "bengaluru"]);
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod fold;
pub mod legacy;
pub mod postgres;
pub mod stem;
#[cfg(feature = "tantivy")]
pub mod tantivy;

use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;

/// 1 search token: a key, and where the word it came from sits in the text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    /// The search key.
    pub text: String,
    /// Byte range of the word in the analyzed text. For legacy text, the range of the
    /// legacy word; for a word split into a number and Kannada, the whole word.
    pub offsets: Range<usize>,
    /// The token's position: 0 for the first token, 1 more for each next one.
    pub position: usize,
}

/// Turns text into search tokens. Use the same analyzer for documents and queries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Analyzer {
    stem: bool,
    legacy: bool,
}

impl Default for Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer {
    /// An analyzer with every step on.
    pub const fn new() -> Self {
        Self {
            stem: true,
            legacy: true,
        }
    }

    /// Turns removal of case and plural endings on or off. Off, a token is the word's
    /// spelling key alone.
    pub const fn stemming(mut self, on: bool) -> Self {
        self.stem = on;
        self
    }

    /// Turns conversion of legacy font lines on or off.
    pub const fn legacy(mut self, on: bool) -> Self {
        self.legacy = on;
        self
    }

    /// The search tokens of `text`, in order.
    pub fn analyze(&self, text: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut line_start = 0;
        for line in text.split_inclusive('\n') {
            match self.legacy.then(|| legacy::detect(line)).flatten() {
                Some(layout) => {
                    for (range, unicode) in legacy::convert_words(line, layout) {
                        let offsets = line_start + range.start..line_start + range.end;
                        for word in words(&unicode) {
                            self.push_word(&mut tokens, word, offsets.clone());
                        }
                    }
                }
                None => {
                    for (start, word) in words_with_offsets(line) {
                        let offsets = line_start + start..line_start + start + word.len();
                        self.push_word(&mut tokens, word, offsets);
                    }
                }
            }
            line_start += line.len();
        }
        tokens
    }

    /// The search keys of `text`, in order.
    pub fn keys(&self, text: &str) -> Vec<String> {
        self.analyze(text).into_iter().map(|t| t.text).collect()
    }

    fn push_word(&self, tokens: &mut Vec<Token>, word: &str, offsets: Range<usize>) {
        let stem = self.stem;
        let push = |keys: &[String], tokens: &mut Vec<Token>| {
            for text in keys {
                tokens.push(Token {
                    text: text.clone(),
                    offsets: offsets.clone(),
                    position: tokens.len(),
                });
            }
        };
        CACHE.with_borrow_mut(|cache| {
            if let Some(keys) = cache[usize::from(stem)].get(word) {
                push(keys, tokens);
                return;
            }
            let folded = fold::fold(word);
            let keys: Vec<String> = script_runs(&folded)
                .into_iter()
                .map(|part| {
                    if stem {
                        stem::stem(part)
                    } else {
                        part.to_string()
                    }
                })
                .collect();
            push(&keys, tokens);
            if word.len() > CACHE_WORD_BYTES {
                return;
            }
            let map = &mut cache[usize::from(stem)];
            if map.len() >= CACHE_WORDS {
                map.clear();
            }
            map.insert(word.to_string(), keys);
        });
    }
}

/// Words seen most recently by this thread and their keys, without and with stemming.
/// Text repeats words often, and a word's keys depend only on the word.
type WordCache = [HashMap<String, Vec<String>>; 2];

/// How many words each cache holds before it is emptied.
const CACHE_WORDS: usize = 1 << 16;

/// Longer words are not cached: real words are shorter, and a long run of letters
/// with no space would fill the cache.
const CACHE_WORD_BYTES: usize = 128;

thread_local! {
    static CACHE: RefCell<WordCache> = RefCell::new([HashMap::new(), HashMap::new()]);
}

/// The words of `text` at UAX #29 boundaries, with their byte offsets. A zero width
/// space between 2 Kannada letters or signs joins them, as the profile's rule S2
/// deletes it there (UAX #29 breaks at it).
fn words_with_offsets(text: &str) -> Vec<(usize, &str)> {
    let mut words: Vec<(usize, &str)> = Vec::new();
    let mut joining = false;
    for (start, segment) in text.split_word_bound_indices() {
        if segment == "\u{200B}" {
            joining = words.last().is_some_and(|&(s, w)| {
                s + w.len() == start && w.chars().last().is_some_and(is_kannada_or_joiner)
            });
            continue;
        }
        let is_word = segment.chars().any(char::is_alphanumeric);
        if joining && is_word && segment.starts_with(is_kannada_or_joiner) {
            let (s, _) = words.pop().expect("joining follows a word");
            words.push((s, &text[s..start + segment.len()]));
        } else if is_word {
            words.push((start, segment));
        }
        joining = false;
    }
    words
}

fn is_kannada_or_joiner(c: char) -> bool {
    kannadi::chars::is_kannada(c) || matches!(c, '\u{200C}' | '\u{200D}')
}

fn words(text: &str) -> impl Iterator<Item = &str> {
    words_with_offsets(text).into_iter().map(|(_, w)| w)
}

/// The searchable parts of a folded word: runs of Kannada and runs of other letters
/// and digits. Kannada written straight after a number or a word in another script
/// is its case ending, as in 2026ರಲ್ಲಿ or Facebookನಲ್ಲಿ, and is dropped.
fn script_runs(word: &str) -> Vec<&str> {
    let mut runs: Vec<&str> = Vec::new();
    let mut start = 0;
    let mut previous: Option<char> = None;
    for (i, c) in word.char_indices() {
        if previous.is_some_and(|p| kannadi::chars::is_kannada(p) != kannadi::chars::is_kannada(c))
        {
            runs.push(&word[start..i]);
            start = i;
        }
        previous = Some(c);
    }
    if start < word.len() {
        runs.push(&word[start..]);
    }
    let mut kept = Vec::new();
    let mut after_other_script = false;
    for run in runs {
        let kannada = run.chars().next().is_some_and(kannadi::chars::is_kannada);
        // Punctuation inside a word, as in O'ಕ್ಲಾಕ್, stays out of the tokens.
        let trimmed =
            run.trim_matches(|c: char| !c.is_alphanumeric() && !kannadi::chars::is_kannada(c));
        if !(kannada && after_other_script) && trimmed.chars().any(char::is_alphanumeric) {
            kept.push(trimmed);
        }
        after_other_script = !kannada && run.chars().last().is_some_and(char::is_alphanumeric);
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offsets_point_at_the_word() {
        let text = "ಹಳೆಯ ಮರಗಳಲ್ಲಿ\nಹಕ್ಕಿ";
        let tokens = Analyzer::new().analyze(text);
        let words: Vec<&str> = tokens.iter().map(|t| &text[t.offsets.clone()]).collect();
        assert_eq!(words, ["ಹಳೆಯ", "ಮರಗಳಲ್ಲಿ", "ಹಕ್ಕಿ"]);
        assert_eq!(
            tokens.iter().map(|t| t.position).collect::<Vec<_>>(),
            [0, 1, 2]
        );
    }

    #[test]
    fn legacy_lines_map_to_their_words() {
        let text = "Act 2005\nPÀ£ÀßqÀ ¨sÁµÉ";
        let tokens = Analyzer::new().analyze(text);
        let pairs: Vec<(&str, &str)> = tokens
            .iter()
            .map(|t| (t.text.as_str(), &text[t.offsets.clone()]))
            .collect();
        assert_eq!(
            pairs,
            [
                ("act", "Act"),
                ("2005", "2005"),
                ("ಕನ್ನಡ", "PÀ£ÀßqÀ"),
                ("ಭಾಷೆ", "¨sÁµÉ")
            ]
        );
        assert!(!Analyzer::new()
            .legacy(false)
            .keys("PÀ£ÀßqÀ")
            .contains(&"ಕನ್ನಡ".to_string()));
    }

    #[test]
    fn joiners_and_signs_stay_inside_words() {
        let keys = Analyzer::new()
            .stemming(false)
            .keys("ಕನ್\u{200C}ನಡ ರಾಜ್\u{200D}ಕುಮಾರ್ ಕಿ\u{0CD5}ರ್ತಿ");
        assert_eq!(keys, ["ಕನ್ನಡ", "ರಾಜ್ಕುಮಾರ್", "ಕೀರ್ತಿ"]);
    }

    #[test]
    fn a_zero_width_space_inside_a_kannada_word_joins_it() {
        let analyzer = Analyzer::new().stemming(false);
        assert_eq!(analyzer.keys("ಕನ್\u{200B}ನಡ"), ["ಕನ್ನಡ"]);
        assert_eq!(analyzer.keys("ಕನ್\u{200C}\u{200B}ನಡ"), ["ಕನ್ನಡ"]);
        assert_eq!(analyzer.keys("ಕನ್ನಡ\u{200B}word"), ["ಕನ್ನಡ", "word"]);
        assert_eq!(analyzer.keys("ಕನ್ನಡ \u{200B}ನಾಡು"), ["ಕನ್ನಡ", "ನಾಡು"]);
    }

    #[test]
    fn numbers_split_from_their_endings() {
        assert_eq!(script_runs("2026ರಲ್ಲಿ"), ["2026"]);
        assert_eq!(script_runs("ಕೊಠಡಿ01"), ["ಕೊಠಡಿ", "01"]);
        assert_eq!(script_runs("ಎಸ್ಎಸ್ಎಲ್ಸಿ"), ["ಎಸ್ಎಸ್ಎಲ್ಸಿ"]);
        assert_eq!(script_runs("covid19ರಿಂದ"), ["covid19"]);
        assert_eq!(script_runs("facebookನಲ್ಲಿ"), ["facebook"]);
        assert_eq!(script_runs("o'ಕ್ಲಾಕ್"), ["o", "ಕ್ಲಾಕ್"]);
        assert_eq!(script_runs("१ರಲ್ಲಿ"), ["१"]);
        assert_eq!(script_runs("ಹೊಂದಿದೆ"), ["ಹೊಂದಿದೆ"]);
    }

    #[test]
    fn stemming_is_optional() {
        assert_eq!(Analyzer::new().keys("ಮರಗಳಲ್ಲಿ"), ["ಮರ"]);
        assert_eq!(Analyzer::new().stemming(false).keys("ಮರಗಳಲ್ಲಿ"), ["ಮರಗಳಲ್ಲಿ"]);
    }

    #[test]
    fn punctuation_and_dandas_are_not_tokens() {
        assert_eq!(Analyzer::new().keys("ಕನ್ನಡ। “ನಾಡಿಗೆ”, - ."), ["ಕನ್ನಡ", "ನಾಡ್"]);
    }
}
