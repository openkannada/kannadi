//! Light stemming: a Kannada noun with case and plural endings becomes its stem.
//!
//! The endings are kannada-morph's noun paradigms (the kannada-spell project),
//! read off as affix rules and kept in `data/suffixes.tsv`: each rule turns a stem
//! with a given final into a form, `stem - strip + add`. They cover case, plural,
//! the clitics ಊ and ಏ, and ಆಗು and ಇರು written fused to a noun (ಸ್ಪಷ್ಟವಾಗಿ,
//! ಮನೆಯಲ್ಲಿದೆ), which Kannada writes as 1 word. [`stem`] runs the rules backwards.
//! It knows no dictionary, so it is careful in 4 ways:
//!
//! - the stem keeps at least 2 aksharas, so short words, where an ending is most
//!   often part of the word, are left alone;
//! - words in `data/protected.txt`, which look inflected but are stems in their
//!   own right (ಮೆರವಣಿಗೆ is not ಮೆರವಣಿ with a dative ending), are left alone;
//! - when endings give more than 1 stem (ವ್ಯಕ್ತಿಗೆ reads as ವ್ಯಕ್ತು + ಇಗೆ or
//!   ವ್ಯಕ್ತಿ + ಗೆ), the stem Kannada Wikipedia uses most as a word wins, from the
//!   counts in `data/preferred.tsv`;
//! - otherwise the longest ending wins, and when it gives more than 1 stem, the word
//!   is left alone.
//!
//! Verbs are out of scope. The result is a search key and need not be a dictionary
//! headword: a final "ು" after a consonant becomes a virama, so a loanword and its
//! "ು" form (ಬ್ಯಾಂಕ್, ಬ್ಯಾಂಕು) fold together, and ಕಾಡು's key is ಕಾಡ್. Stemming a
//! key again gives the same key.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use kannadi::chars::{is_consonant, is_vowel_sign, NUKTA, VIRAMA};
use kannadi::count_aksharas;

/// The fewest aksharas a stem keeps.
const MIN_STEM_AKSHARAS: usize = 2;

const SUFFIXES: &str = include_str!("../data/suffixes.tsv");
const PROTECTED: &str = include_str!("../data/protected.txt");
const PREFERRED: &str = include_str!("../data/preferred.tsv");

/// The stem of a Kannada word given in its search key ([`crate::fold::fold`]).
/// Words that are not Kannada are returned as they are.
///
/// ```
/// use kannadi_search::stem::stem;
///
/// assert_eq!(stem("ಮರಗಳಲ್ಲಿ"), "ಮರ");
/// assert_eq!(stem("ಮನೆಯನ್ನು"), "ಮನೆ");
/// assert_eq!(stem("ಕಾಡಿನಲ್ಲಿ"), stem("ಕಾಡು"));
/// assert_eq!(stem("ಬ್ಯಾಂಕ್ನಲ್ಲಿ"), stem("ಬ್ಯಾಂಕಿನ"));
/// assert_eq!(stem("ಮೆರವಣಿಗೆ"), "ಮೆರವಣಿಗೆ");
/// ```
pub fn stem(word: &str) -> String {
    if word.is_empty() || !word.chars().all(kannadi::chars::is_kannada) {
        return word.to_string();
    }
    reduce(word, true)
}

/// The key `word` reduces to, removing 1 ending per round. Each round starts from the
/// stem as the rule spelled it: ಮನೆಯನ್ನಾಗಿ gives ಮನೆಯನ್ನು, then ಮನೆ. Its key, ಮನೆಯನ್ನ್,
/// would end the search, as no ending ends in a virama.
fn reduce(word: &str, protect: bool) -> String {
    rounds(word, protect).pop().unwrap_or_else(|| key(word))
}

/// The key after each round of [`reduce`].
fn rounds(word: &str, protect: bool) -> Vec<String> {
    let mut spelled = word.to_string();
    let mut keys = Vec::new();
    for _ in 0..MAX_ROUNDS {
        if protect && is_protected(&spelled) {
            break;
        }
        match strip(&spelled) {
            Some(next) if key(&next) != key(&spelled) => {
                keys.push(key(&next));
                spelled = next;
            }
            _ => break,
        }
    }
    keys
}

/// How many times an ending is removed at most: case on plural on a fused ಆಗು is 3.
const MAX_ROUNDS: usize = 4;

/// The preferred stem with the highest count among the [`candidates`] of `word`; else
/// the stem the longest ending leaves, if it leaves only 1. The stem is spelled as the
/// rule leaves it.
fn strip(word: &str) -> Option<String> {
    let readings = spelled_readings(word);
    let preferred = preferred();
    let mut best: Option<(&String, u64)> = None;
    for stem in readings.iter().flatten() {
        // The list holds only stems no ending shortens (see `candidates`).
        if let Some(&n) = preferred.get(key(stem).as_str()) {
            if best.is_none_or(|(_, b)| n > b) {
                best = Some((stem, n));
            }
        }
    }
    if let Some((stem, _)) = best {
        return Some(stem.clone());
    }
    match readings.first().map(Vec::as_slice) {
        Some([only]) => Some(only.clone()),
        _ => None,
    }
}

#[cfg(feature = "build-data")]
/// Every form the rules make from `stem`, other than `stem` itself.
fn forms(stem: &str) -> Vec<String> {
    let mut forms: Vec<String> = rules()
        .by_add
        .iter()
        .flat_map(|(add, rules)| rules.iter().map(move |r| (add, r)))
        .filter(|(_, r)| r.accepts(stem))
        .filter_map(|(add, r)| Some(format!("{}{add}", stem.strip_suffix(r.strip)?)))
        .collect();
    forms.sort();
    forms.dedup();
    forms
}

#[cfg(any(test, feature = "build-data"))]
/// True when no ending shortens any spelling of `key`.
fn is_final(key: &str) -> bool {
    spellings(key).all(|s| spelled_readings(&s).is_empty())
}

#[cfg(any(test, feature = "build-data"))]
/// The stems of `word` that no ending shortens further in any spelling, as keys,
/// longest ending first. A plural such as ಅಂಶಗಳು (key ಅಂಶಗಳ್) is not a candidate for
/// ಅಂಶಗಳನ್ನು: Wikipedia uses it often as a word, but it is itself a form of ಅಂಶ.
fn candidates(word: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for stem in spelled_readings(word).into_iter().flatten() {
        let key = key(&stem);
        if is_final(&key) && !out.contains(&key) {
            out.push(key);
        }
    }
    out
}

#[cfg(any(test, feature = "build-data"))]
/// The spellings with this key: ಕಾಡ್ and ಕಾಡು for ಕಾಡ್.
fn spellings(key: &str) -> impl Iterator<Item = String> {
    let u = key
        .strip_suffix(VIRAMA)
        .filter(|body| body.chars().last().is_some_and(is_consonant))
        .map(|body| format!("{body}ು"));
    std::iter::once(key.to_string()).chain(u)
}

fn preferred() -> &'static HashMap<&'static str, u64> {
    static STEMS: OnceLock<HashMap<&'static str, u64>> = OnceLock::new();
    STEMS.get_or_init(|| {
        PREFERRED
            .lines()
            .filter(|l| !l.starts_with('#'))
            .map(|l| {
                let (stem, n) = l
                    .split_once('\t')
                    .expect("data/preferred.tsv: stem and count");
                (stem, n.parse().expect("data/preferred.tsv: count"))
            })
            .collect()
    })
}

#[cfg(any(test, feature = "build-data"))]
/// The stems each matching ending leaves, as [`key`]s, longest ending first: 1 list
/// per ending length, each key once. A reading that gives the word's own key back
/// (ಕಾಡು as the loanword ಕಾಡ್ with "ು") is not a reading.
fn readings(word: &str) -> Vec<Vec<String>> {
    spelled_readings(word)
        .into_iter()
        .map(|stems| stems.iter().map(|s| key(s)).collect())
        .collect()
}

/// [`readings`], with each stem spelled as the rule leaves it (ಕಾಡು, before its key ಕಾಡ್).
fn spelled_readings(word: &str) -> Vec<Vec<String>> {
    let rules = rules();
    let word_key = key(word);
    let mut keys: Vec<String> = Vec::new();
    let mut out: Vec<Vec<String>> = Vec::new();
    let shortest_head = word.len().saturating_sub(rules.longest_add);
    for (i, _) in word
        .char_indices()
        .skip(1)
        .filter(|&(i, _)| i >= shortest_head)
    {
        let (head, add) = word.split_at(i);
        let mut stems: Vec<String> = Vec::new();
        for rule in rules.by_add.get(add).into_iter().flatten() {
            let stem = format!("{head}{}", rule.strip);
            if rule.accepts(&stem)
                && count_aksharas(&stem) >= MIN_STEM_AKSHARAS
                && is_spelled_right(&stem)
            {
                let stem_key = key(&stem);
                if stem_key == word_key {
                    continue;
                }
                if !keys.contains(&stem_key) {
                    keys.push(stem_key);
                    stems.push(stem);
                } else if stem.ends_with('ು') {
                    // Keep the "ು" spelling of a key: endings can still follow it.
                    for s in out.iter_mut().chain([&mut stems]).flatten() {
                        if key(s) == stem_key {
                            *s = stem.clone();
                        }
                    }
                }
            }
        }
        if !stems.is_empty() {
            out.push(stems);
        }
    }
    out
}

/// True when every vowel sign and virama follows a consonant (or its nukta). A typo
/// such as ಮೈದಾನದಲ್ಲ್ಲಿ would otherwise give the stem ಮೈದಾನದಲ್ು.
fn is_spelled_right(stem: &str) -> bool {
    let mut previous = None;
    stem.chars().all(|c| {
        let ok = !(c == VIRAMA || is_vowel_sign(c))
            || previous.is_some_and(|p| is_consonant(p) || p == NUKTA);
        previous = Some(c);
        ok
    })
}

/// 1 spelling for stems that Kannada writes 2 ways: a final consonant with "ು"
/// (ಬ್ಯಾಂಕು, ಕಾಡು) becomes that consonant with a virama (ಬ್ಯಾಂಕ್, ಕಾಡ್), as a
/// loanword is also written. No ending ends in a virama, so a key is never cut again.
fn key(stem: &str) -> String {
    match stem.strip_suffix('ು') {
        Some(body) if body.chars().last().is_some_and(is_consonant) => format!("{body}{VIRAMA}"),
        _ => stem.to_string(),
    }
}

/// What the tool in `eval/` needs to rebuild `data/preferred.tsv` and
/// `data/protected.txt`. Not a stable interface.
#[cfg(feature = "build-data")]
pub mod build {
    /// The keys each matching ending leaves, longest ending first.
    pub fn readings(word: &str) -> Vec<Vec<String>> {
        super::readings(word)
    }

    /// The keys of [`readings`] that no ending shortens further.
    pub fn candidates(word: &str) -> Vec<String> {
        super::candidates(word)
    }

    /// Every form the rules make from a stem.
    pub fn forms(stem: &str) -> Vec<String> {
        super::forms(stem)
    }

    /// The spellings with a key: ಕಾಡ್ and ಕಾಡು for ಕಾಡ್.
    pub fn spellings(key: &str) -> Vec<String> {
        super::spellings(key).collect()
    }

    /// A stem's key.
    pub fn key(stem: &str) -> String {
        super::key(stem)
    }

    /// The key after each round of stemming, ignoring the protected words.
    pub fn rounds(word: &str) -> Vec<String> {
        super::rounds(word, false)
    }
}

fn is_protected(word: &str) -> bool {
    static WORDS: OnceLock<HashSet<&'static str>> = OnceLock::new();
    WORDS
        .get_or_init(|| {
            PROTECTED
                .lines()
                .filter(|l| !l.starts_with('#') && !l.is_empty())
                .collect()
        })
        .contains(word)
}

/// The final a stem must have for a rule to apply.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Final {
    /// A consonant with its inherent vowel: ಮರ.
    Consonant,
    /// These last characters: a vowel sign, the virama, or ದು.
    Ends(&'static str),
}

#[derive(Debug)]
struct Rule {
    strip: &'static str,
    finals: Vec<Final>,
}

impl Rule {
    fn accepts(&self, stem: &str) -> bool {
        let Some(last) = stem.chars().last() else {
            return false;
        };
        self.finals.iter().any(|f| match *f {
            Final::Consonant => is_consonant(last),
            Final::Ends(f) => stem.ends_with(f),
        })
    }
}

struct Rules {
    by_add: HashMap<&'static str, Vec<Rule>>,
    /// The length of the longest ending, in bytes.
    longest_add: usize,
}

fn rules() -> &'static Rules {
    static RULES: OnceLock<Rules> = OnceLock::new();
    RULES.get_or_init(|| {
        let mut by_add: HashMap<&'static str, Vec<Rule>> = HashMap::new();
        for line in SUFFIXES.lines().filter(|l| !l.starts_with('#')).skip(1) {
            let mut fields = line.split('\t');
            let (Some(_class), Some(strip), Some(add), Some(finals)) =
                (fields.next(), fields.next(), fields.next(), fields.next())
            else {
                panic!("data/suffixes.tsv: bad row {line:?}");
            };
            if add.is_empty() {
                continue;
            }
            let finals = finals.split(',').map(parse_final).collect();
            by_add.entry(add).or_default().push(Rule { strip, finals });
        }
        let longest_add = by_add.keys().map(|a| a.len()).max().unwrap_or(0);
        Rules {
            by_add,
            longest_add,
        }
    })
}

fn parse_final(f: &'static str) -> Final {
    match f {
        "C" => Final::Consonant,
        "" => panic!("data/suffixes.tsv: empty final"),
        f => Final::Ends(f),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same_stem(words: &[&str]) {
        let stems: Vec<String> = words.iter().map(|w| stem(w)).collect();
        assert!(
            stems.windows(2).all(|p| p[0] == p[1]),
            "{words:?} -> {stems:?}"
        );
    }

    #[test]
    fn the_most_used_reading_wins() {
        assert_eq!(stem("ಅಭಿವೃದ್ಧಿಗೆ"), "ಅಭಿವೃದ್ಧಿ");
        assert_eq!(stem("ಕಾಡಿಗೆ"), "ಕಾಡ್");
        assert!(preferred().len() > 1000);
    }

    #[test]
    fn endings_come_off_1_at_a_time_from_the_spelled_stem() {
        // ಮನೆಯನ್ನಾಗಿ: ಮನೆಯನ್ನು + ಆಗಿ, then ಮನೆ + ಅನ್ನು.
        assert_eq!(stem("ಮನೆಯನ್ನಾಗಿ"), "ಮನೆ");
        assert_eq!(stem("ಸಂಸ್ಥೆಯನ್ನಾಗಿ"), stem("ಸಂಸ್ಥೆ"));
        same_stem(&["ಆಳ್ವಾರರು", "ಆಳ್ವಾರರುಗಳು"]);
    }

    #[test]
    fn inflected_forms_are_not_protected() {
        assert_eq!(stem("ಅಂಶವಾಗಿದ್ದ"), "ಅಂಶ");
        assert_eq!(stem("ಸಾಧನವನ್ನು"), "ಸಾಧನ");
        assert_eq!(stem("ಉಡುಗೊರೆಯನ್ನು"), "ಉಡುಗೊರೆ");
    }

    #[test]
    fn a_word_with_2_readings_of_its_longest_ending_stays_whole() {
        // ವೃಕೋದ + ರ (plural genitive) or ವೃಕೋದು + ರ (genitive of ಇದು-type words).
        assert_eq!(readings("ವೃಕೋದರ")[0].len(), 2);
        assert_eq!(stem("ವೃಕೋದರ"), "ವೃಕೋದರ");
    }

    #[test]
    fn a_word_is_not_its_own_reading() {
        assert!(readings("ಕಾಡು").is_empty());
        assert!(candidates("ಕಾಡಿಗೆ").contains(&"ಕಾಡ್".to_string()));
    }

    #[test]
    fn listed_stems_are_final() {
        for stem in preferred().keys() {
            assert!(is_final(stem), "{stem}");
        }
    }

    #[test]
    fn the_table_parses() {
        assert!(rules().by_add.len() > 100);
        assert!(rules().by_add.contains_key("ಗಳನ್ನು"));
    }

    #[test]
    fn case_and_plural_forms_share_a_stem() {
        same_stem(&["ಮರ", "ಮರವನ್ನು", "ಮರಕ್ಕೆ", "ಮರದಲ್ಲಿ", "ಮರಗಳು", "ಮರಗಳಿಗೆ", "ಮರದಿಂದ"]);
        same_stem(&["ಮನೆ", "ಮನೆಯನ್ನು", "ಮನೆಗೆ", "ಮನೆಯಲ್ಲಿ", "ಮನೆಗಳಲ್ಲಿ", "ಮನೆಯೂ"]);
        same_stem(&["ಕಾಡು", "ಕಾಡನ್ನು", "ಕಾಡಿಗೆ", "ಕಾಡಿನಲ್ಲಿ", "ಕಾಡುಗಳು"]);
        same_stem(&["ವಸ್ತು", "ವಸ್ತುವನ್ನು", "ವಸ್ತುವಿಗೆ", "ವಸ್ತುಗಳು"]);
        same_stem(&["ಹುಡುಗ", "ಹುಡುಗನನ್ನು", "ಹುಡುಗನಿಗೆ", "ಹುಡುಗರು"]);
        same_stem(&["ವ್ಯಕ್ತಿ", "ವ್ಯಕ್ತಿಗೆ", "ವ್ಯಕ್ತಿಯನ್ನು", "ವ್ಯಕ್ತಿಗಳು"]);
        same_stem(&["ಸ್ಪಷ್ಟ", "ಸ್ಪಷ್ಟವಾಗಿ", "ಸ್ಪಷ್ಟವಾದ"]);
        same_stem(&["ಮನೆ", "ಮನೆಯಲ್ಲಿದೆ", "ಮನೆಯಲ್ಲಿದ್ದ"]);
        same_stem(&["ಬ್ಯಾಂಕ್", "ಬ್ಯಾಂಕ್ನಲ್ಲಿ", "ಬ್ಯಾಂಕಿನ", "ಬ್ಯಾಂಕುಗಳು"]);
        same_stem(&["ಬಸ್ಸು", "ಬಸ್ಸಿನಲ್ಲಿ", "ಬಸ್ಸುಗಳು"]);
        same_stem(&["ಹಬ್ಬ", "ಹಬ್ಬದ", "ಹಬ್ಬಗಳು"]);
    }

    #[test]
    fn different_words_keep_different_stems() {
        assert_ne!(stem("ಮಗನನ್ನು"), stem("ಮಗುವನ್ನು"));
        assert_ne!(stem("ಮರ"), stem("ಮನೆ"));
        assert_ne!(stem("ಹಣ"), stem("ಹಣ್ಣು"));
    }

    #[test]
    fn short_stems_are_left_alone() {
        assert_eq!(stem("ಪದ"), "ಪದ");
        assert_eq!(stem("ಬಗೆ"), "ಬಗೆ");
    }

    #[test]
    fn other_scripts_are_left_alone() {
        assert_eq!(stem("trees"), "trees");
        assert_eq!(stem("2026"), "2026");
        assert_eq!(stem(""), "");
    }

    #[test]
    fn misspelled_stems_are_not_readings() {
        assert!(is_spelled_right("ಮೈದಾನ"));
        assert!(!is_spelled_right("ಮೈದಾನದಲ್ು"));
        assert!(!is_spelled_right("ಕಾಾ"));
        assert_eq!(stem("ಮೈದಾನದಲ್ಲ್ಲಿ"), stem(&stem("ಮೈದಾನದಲ್ಲ್ಲಿ")));
    }

    #[test]
    fn loanword_finals_fold() {
        assert_eq!(key("ಬ್ಯಾಂಕು"), "ಬ್ಯಾಂಕ್");
        assert_eq!(key("ಬ್ಯಾಂಕ್"), "ಬ್ಯಾಂಕ್");
        assert_eq!(key("ಮರ"), "ಮರ");
        assert_eq!(key("ು"), "ು");
        assert_eq!(key("ಊರೂ"), "ಊರೂ");
    }
}
