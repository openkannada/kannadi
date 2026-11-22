//! What 1 user has picked: kept in memory, saved and restored as text by the
//! caller. Nothing here touches storage or the network.

use crate::units::units;
use fst::Set;
use std::collections::{BTreeSet, HashMap};

/// At most this many (input, word) picks are kept; the oldest go first.
pub const MAX_PICKS: usize = 2000;
/// At most this many picked words are counted; the least picked go first.
pub const MAX_WORDS: usize = 5000;
const HEADER: &str = "kannadi-typing user 1";

#[derive(Default)]
pub struct User {
    /// Oldest first, each (input, word) once.
    picks: Vec<(String, String)>,
    counts: HashMap<String, u32>,
    /// Picked words the lexicon lacks, as unit bytes.
    extra: BTreeSet<Vec<u8>>,
    extra_set: Option<Set<Vec<u8>>>,
}

impl User {
    /// Records that `word` was chosen for `input`. `known` says whether the lexicon has the word.
    pub fn learn(&mut self, input: &str, word: &str, known: bool) -> bool {
        let Some(u) = units(word) else { return false };
        let word = crate::units::text(&u);
        let input = input.to_ascii_lowercase();
        self.picks.retain(|(i, w)| !(i == &input && w == &word));
        self.picks.push((input, word.clone()));
        if self.picks.len() > MAX_PICKS {
            self.picks.remove(0);
        }
        let count = self.counts.entry(word.clone()).or_default();
        *count = count.saturating_add(1);
        if self.counts.len() > MAX_WORDS {
            if let Some(least) = self
                .counts
                .iter()
                .filter(|(w, _)| **w != word)
                .min_by(|a, b| a.1.cmp(b.1).then(b.0.cmp(a.0)))
                .map(|(w, _)| w.clone())
            {
                self.forget(&least);
            }
        }
        if !known && self.counts.contains_key(&word) && self.extra.insert(u) {
            self.rebuild();
        }
        true
    }

    /// Forgets every pick of `word`.
    pub fn forget(&mut self, word: &str) {
        let Some(u) = units(word) else { return };
        let word = crate::units::text(&u);
        self.picks.retain(|(_, w)| w != &word);
        self.counts.remove(&word);
        if self.extra.remove(&u) {
            self.rebuild();
        }
    }

    fn rebuild(&mut self) {
        self.extra_set = if self.extra.is_empty() {
            None
        } else {
            Set::from_iter(self.extra.iter()).ok()
        };
    }

    /// The words chosen before for exactly `input`, newest last.
    pub fn picks_for<'a>(&'a self, input: &'a str) -> impl Iterator<Item = (usize, &'a str)> + 'a {
        let input = input.to_ascii_lowercase();
        self.picks
            .iter()
            .enumerate()
            .filter(move |(_, (i, _))| *i == input)
            .map(|(rank, (_, w))| (rank, w.as_str()))
    }

    /// How many times `word` was picked.
    pub fn count(&self, word: &str) -> u32 {
        self.counts.get(word).copied().unwrap_or(0)
    }

    /// The picked words the lexicon lacks, as a lexicon of their own.
    pub fn extra(&self) -> Option<&Set<Vec<u8>>> {
        self.extra_set.as_ref()
    }

    /// The picks as text, for the caller to store.
    pub fn export(&self) -> String {
        let mut out = String::from(HEADER);
        out.push('\n');
        for (i, w) in &self.picks {
            out.push_str(&format!("p\t{i}\t{w}\n"));
        }
        let mut counts: Vec<_> = self.counts.iter().collect();
        counts.sort();
        for (w, c) in counts {
            let extra = units(w).is_some_and(|u| self.extra.contains(&u));
            out.push_str(&format!("{}\t{w}\t{c}\n", if extra { "x" } else { "c" }));
        }
        out
    }

    /// Restores picks written by [`User::export`], replacing the current ones.
    /// Returns `None`, and changes nothing, if `text` is not such an export.
    pub fn import(text: &str) -> Option<User> {
        let mut lines = text.lines();
        if lines.next()? != HEADER {
            return None;
        }
        let mut user = User::default();
        for line in lines {
            let mut f = line.split('\t');
            let (Some(kind), Some(a), Some(b), None) = (f.next(), f.next(), f.next(), f.next())
            else {
                continue;
            };
            let valid_input = !a.is_empty() && a.bytes().all(|c| c.is_ascii_alphabetic());
            match kind {
                "p" if valid_input && units(b).is_some() && user.picks.len() < MAX_PICKS => {
                    user.picks.push((a.to_ascii_lowercase(), b.to_string()));
                }
                "c" | "x" if user.counts.len() < MAX_WORDS => {
                    let (Some(u), Ok(c)) = (units(a), b.parse::<u32>()) else {
                        continue;
                    };
                    user.counts.insert(crate::units::text(&u), c);
                    if kind == "x" {
                        user.extra.insert(u);
                    }
                }
                _ => {}
            }
        }
        user.rebuild();
        Some(user)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_store_keeps_the_word_just_learned() {
        let mut u = User::default();
        let consonants = [
            "ಕ", "ಗ", "ಚ", "ಜ", "ಟ", "ಡ", "ತ", "ದ", "ನ", "ಪ", "ಬ", "ಮ", "ಯ", "ರ", "ಲ", "ವ", "ಸ",
            "ಹ",
        ];
        let mut n = 0;
        'fill: for a in consonants {
            for b in consonants {
                for c in consonants {
                    if n == MAX_WORDS {
                        break 'fill;
                    }
                    assert!(u.learn("abc", &format!("{a}{b}{c}"), true));
                    n += 1;
                }
            }
        }
        assert!(u.learn("hahaha", "ೞೞ", false));
        assert_eq!(u.count("ೞೞ"), 1);
        assert!(u.extra().is_some());
        assert_eq!(u.counts.len(), MAX_WORDS);
    }

    #[test]
    fn counts_stop_at_the_largest_value() {
        let mut u = User::import(&format!("{HEADER}\nc\tಕನ್ನಡ\t{}\n", u32::MAX)).unwrap();
        assert!(u.learn("kannada", "ಕನ್ನಡ", true));
        assert_eq!(u.count("ಕನ್ನಡ"), u32::MAX);
    }
}
