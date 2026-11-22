//! Top-k accuracy on a test set, as IndicXlit and the NEWS shared tasks score it.
//!
//! Each distinct romanization (ignoring case) is 1 test item. Its references are
//! every native word the test set gives for it. A prediction is correct when
//! it equals 1 of the references after removing joiners.

use crate::units::{text, units};
use crate::{Candidate, Engine, Weights};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// 1 test item.
#[derive(Clone, Debug)]
pub struct Item {
    pub input: String,
    pub prev: Option<String>,
    pub refs: Vec<String>,
}

fn clean(word: &str) -> String {
    units(word).map_or_else(|| word.to_string(), |u| text(&u))
}

/// Groups (native, romanization) rows into items, 1 per romanization.
pub fn items(rows: &[(String, String)]) -> Vec<Item> {
    let mut by_input: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (native, roman) in rows {
        let refs = by_input.entry(roman.to_lowercase()).or_default();
        let native = clean(native);
        if !refs.contains(&native) {
            refs.push(native);
        }
    }
    by_input
        .into_iter()
        .map(|(input, refs)| Item {
            input,
            prev: None,
            refs,
        })
        .collect()
}

/// Items from parallel native and romanized sentences, each word with the
/// native word before it. Sentences whose word counts differ are skipped.
pub fn context_items(native: &[String], roman: &[String]) -> Vec<Item> {
    let mut out = Vec::new();
    for (n, r) in native.iter().zip(roman) {
        let strip = |s: &str| {
            s.trim_matches(|c: char| {
                !c.is_alphanumeric() && !('\u{0C80}'..='\u{0CFF}').contains(&c)
            })
            .to_string()
        };
        let nw: Vec<String> = n.split_whitespace().map(strip).collect();
        let rw: Vec<String> = r.split_whitespace().map(strip).collect();
        if nw.len() != rw.len() {
            continue;
        }
        for i in 0..nw.len() {
            let ok = units(&nw[i]).is_some()
                && !rw[i].is_empty()
                && rw[i].bytes().all(|b| b.is_ascii_alphabetic());
            if ok {
                let prev = i.checked_sub(1).map(|p| clean(&nw[p]));
                out.push(Item {
                    input: rw[i].to_lowercase(),
                    prev,
                    refs: vec![clean(&nw[i])],
                });
            }
        }
    }
    out
}

/// Accuracy at 1 and 5, and how long each call took.
#[derive(Clone, Debug, Default)]
pub struct Report {
    pub items: usize,
    pub top1: usize,
    pub top5: usize,
    /// Items whose first reference is in the lexicon.
    pub known: usize,
    pub known_top1: usize,
    pub times: Vec<Duration>,
}

impl Report {
    pub fn pct(n: usize, of: usize) -> f64 {
        100.0 * n as f64 / of.max(1) as f64
    }
    /// The time below which `q` of the calls finished.
    pub fn quantile(&self, q: f64) -> Duration {
        let mut t = self.times.clone();
        t.sort();
        t.get(((t.len() as f64 * q) as usize).min(t.len().saturating_sub(1)))
            .copied()
            .unwrap_or_default()
    }
}

/// The candidates for every item, for scoring under many weights.
pub fn candidates(engine: &Engine, items: &[Item]) -> (Vec<Vec<Candidate>>, Vec<Duration>) {
    let mut times = Vec::with_capacity(items.len());
    let lists = items
        .iter()
        .map(|it| {
            let t = Instant::now();
            let c = engine.candidates(&it.input, it.prev.as_deref());
            times.push(t.elapsed());
            c
        })
        .collect();
    (lists, times)
}

/// Scores the candidate lists under `w`.
pub fn score(engine: &Engine, items: &[Item], lists: &[Vec<Candidate>], w: &Weights) -> Report {
    let mut r = Report {
        items: items.len(),
        ..Report::default()
    };
    for (it, list) in items.iter().zip(lists) {
        let mut ranked = list.clone();
        crate::rank(&mut ranked, w, crate::roman::typed_letters(&it.input));
        let rank = ranked.iter().position(|c| it.refs.contains(&c.word));
        r.top1 += usize::from(rank == Some(0));
        r.top5 += usize::from(rank.is_some_and(|k| k < 5));
        let known = units(&it.refs[0]).is_some_and(|u| engine.model().lexicon.get(u).is_some());
        if known {
            r.known += 1;
            r.known_top1 += usize::from(rank == Some(0));
        }
    }
    r
}

/// Coordinate ascent on the ranking weights, maximizing accuracy at 1, then at 5.
pub fn tune(engine: &Engine, items: &[Item], lists: &[Vec<Candidate>]) -> Weights {
    let mut best = engine.model().weights();
    let value = |w: &Weights| {
        let r = score(engine, items, lists, w);
        r.top1 * 10_000 + r.top5
    };
    let mut best_value = value(&best);
    type Field = fn(&mut Weights) -> &mut f32;
    let fields: [(Field, &[f32]); 4] = [
        (
            |w| &mut w.freq,
            &[
                0.0, 0.2, 0.4, 0.6, 0.8, 1.0, 1.2, 1.5, 1.8, 2.2, 2.6, 3.0, 4.0,
            ],
        ),
        (
            |w| &mut w.known,
            &[
                0.0, 2.0, 4.0, 6.0, 8.0, 10.0, 15.0, 20.0, 25.0, 30.0, 40.0, 60.0,
            ],
        ),
        (|w| &mut w.rule, &[-2.0, 0.0, 0.5, 1.0, 2.0, 3.0, 5.0]),
        (|w| &mut w.pair, &[0.0, 0.25, 0.5, 0.75, 1.0, 1.5, 2.0]),
    ];
    for _ in 0..3 {
        for (field, grid) in &fields {
            for &x in grid.iter() {
                let mut w = best;
                *field(&mut w) = x;
                let v = value(&w);
                if v > best_value {
                    best_value = v;
                    best = w;
                }
            }
        }
    }
    best
}

/// Items for the words cut 2 letters short, as typed midway; only inputs of 5 letters or more.
pub fn prefixes(items: &[Item]) -> Vec<Item> {
    items
        .iter()
        .filter(|it| it.input.len() >= 5 && it.input.is_ascii())
        .map(|it| Item {
            input: it.input[..it.input.len() - 2].to_string(),
            ..it.clone()
        })
        .collect()
}

/// The completion weight that puts the intended word in the first 5 for most
/// words typed midway, plus whole words, given the other weights.
pub fn tune_extend(
    engine: &Engine,
    whole: (&[Item], &[Vec<Candidate>]),
    cut: (&[Item], &[Vec<Candidate>]),
    w: Weights,
) -> Weights {
    let mut best = (0, w);
    for x in [
        -12.0, -10.0, -8.0, -7.0, -6.0, -5.0, -4.0, -3.0, -2.0, -1.0, 0.0,
    ] {
        let t = Weights { extend: x, ..w };
        let v = score(engine, whole.0, whole.1, &t).top5 + score(engine, cut.0, cut.1, &t).top5;
        if v > best.0 {
            best = (v, t);
        }
    }
    best.1
}
