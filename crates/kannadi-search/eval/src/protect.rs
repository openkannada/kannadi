//! `protect`: words that end like an inflected noun but are stems of their own.
//!
//! A word w that the endings would cut to s is kept whole when Kannada Wikipedia has at
//! least 2 forms built on w (each seen at least twice) that are not also forms of s, and
//! those forms are together at least 1 tenth as frequent as w itself.
//! ಮೆರವಣಿಗೆಯಲ್ಲಿ and ಮೆರವಣಿಗೆಯನ್ನು show that ಮೆರವಣಿಗೆ is a stem; ಮರಗಳನ್ನು, built on
//! ಮರಗಳು, is also a form of ಮರ and shows nothing. The share keeps genitives such as
//! ಕಾಲದ, seen thousands of times, from counting as stems because of a rare ಕಾಲದವು
//! ("those of the time").

use std::collections::{HashMap, HashSet};

use kannadi_search::stem::{forms, strip_ending};

pub const MIN_COUNT: u64 = 2;
pub const MIN_FORMS: usize = 2;
/// The forms' total count, in tenths of the word's own count.
pub const MIN_SHARE_TENTHS: u64 = 1;

pub fn protected(counts: &HashMap<String, u64>) -> Vec<String> {
    let mut out: Vec<String> = counts
        .iter()
        .filter(|(w, &n)| n >= MIN_COUNT && w.chars().all(kannadi::chars::is_kannada))
        .filter_map(|(w, &n)| {
            let s = strip_ending(w)?;
            let of_s = forms_of_key(&s);
            let evidence: Vec<u64> = forms(w)
                .into_iter()
                .filter(|f| !of_s.contains(f))
                .filter_map(|f| counts.get(&f).copied().filter(|&c| c >= MIN_COUNT))
                .collect();
            let share = evidence.iter().sum::<u64>() * 10 >= n * MIN_SHARE_TENTHS;
            (evidence.len() >= MIN_FORMS && share).then(|| w.clone())
        })
        .collect();
    out.sort();
    out
}

/// The forms of every stem with this key: ಕಾಡ್ is the key of ಕಾಡ್ and ಕಾಡು.
fn forms_of_key(key: &str) -> HashSet<String> {
    kannadi_search::stem::spellings(key)
        .flat_map(|s| forms(&s))
        .collect()
}

pub fn read_counts(text: &str) -> HashMap<String, u64> {
    text.lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let (w, n) = l.split_once('\t')?;
            Some((kannadi_search::fold::fold(w), n.trim().parse::<u64>().ok()?))
        })
        .fold(HashMap::new(), |mut m, (w, n)| {
            // A stem is counted under its key too: ಕಾಡು and ಕಾಡ್ both count for ಕಾಡ್.
            let key = kannadi_search::stem::key(&w);
            if key != w {
                *m.entry(key).or_insert(0) += n;
            }
            *m.entry(w).or_insert(0) += n;
            m
        })
}

/// `prefer`: stems that settle words with more than 1 reading, with their Wikipedia counts.
///
/// For each word, the best reading is the one Wikipedia has most often as a word (seen at
/// least [`MIN_STEM_COUNT`] times); the stemmer's default is the reading of the longest
/// ending, when it is unique. The list holds the best reading of every word where the 2
/// differ, and the default wherever a listed stem would otherwise outvote it, so that
/// "the listed reading with the highest count" gives the best reading for every word.
pub const MIN_STEM_COUNT: u64 = 3;

pub fn preferred(counts: &HashMap<String, u64>) -> Vec<(String, u64)> {
    let words: Vec<(Vec<String>, Option<String>, Option<String>)> = counts
        .iter()
        .filter(|(w, &n)| n >= MIN_COUNT && w.chars().all(kannadi::chars::is_kannada))
        .filter_map(|(w, _)| {
            let groups = kannadi_search::stem::readings(w);
            let default = groups
                .first()
                .filter(|g| g.len() == 1)
                .map(|g| g[0].clone());
            let cands = kannadi_search::stem::candidates(w);
            if cands.is_empty() || (cands.len() == 1 && default.as_ref() == cands.first()) {
                return None;
            }
            let count = |s: &String| counts.get(s).copied().unwrap_or(0);
            let best = cands
                .iter()
                .filter(|s| count(s) >= MIN_STEM_COUNT)
                .fold(None::<&String>, |b, s| match b {
                    Some(b) if count(b) >= count(s) => Some(b),
                    _ => Some(s),
                })
                .cloned()
                .or_else(|| default.clone());
            Some((cands, default, best))
        })
        .collect();
    let mut listed: HashMap<String, u64> = HashMap::new();
    loop {
        let before = listed.len();
        for (cands, default, best) in &words {
            let choice = cands
                .iter()
                .filter(|s| listed.contains_key(*s))
                .fold(None::<&String>, |b, s| match b {
                    Some(b) if listed[b] >= listed[s] => Some(b),
                    _ => Some(s),
                })
                .or(default.as_ref());
            if choice != best.as_ref() {
                if let Some(best) = best {
                    listed.insert(best.clone(), counts.get(best).copied().unwrap_or(0));
                }
            }
        }
        if listed.len() == before {
            break;
        }
    }
    let mut out: Vec<(String, u64)> = listed.into_iter().collect();
    out.sort();
    out
}
