//! Beam search for the Kannada words that best explain a run of Latin letters.
//!
//! The search reads the letters left to right. Each step pairs the next 0 to 3
//! letters with 1 Kannada unit (a graphone) and scores it with the n-gram
//! model. Given a lexicon, a step must also follow an arc of the lexicon
//! automaton, so only words of the lexicon come out.

use crate::complete::Completions;
use crate::lm::MAX_ORDER;
use crate::model::Model;
use crate::roman::MAX_CHUNK;
use crate::units::{may_end, may_follow, Unit};
use fst::raw::{CompiledAddr, Fst};
use std::collections::HashMap;

/// A word the search found.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub units: Vec<Unit>,
    /// Natural log of P(letters, word) under the n-gram model.
    pub lm: f32,
    /// The lexicon value of the word, if searched in a lexicon.
    pub value: Option<u64>,
    /// How many units were added after the letters ran out to complete a lexicon word; 0 for a whole word.
    pub extra: usize,
}

/// At most this many of the best partial readings are completed to whole words.
const COMPLETE: usize = 8;

const HIST: usize = MAX_ORDER - 1;
const NO_NODE: CompiledAddr = usize::MAX;

#[derive(Clone, Copy)]
struct Hyp {
    score: f32,
    hist: [u16; HIST],
    hlen: u8,
    last: Unit,
    node: CompiledAddr,
    out: u64,
    back: u32,
}

impl Hyp {
    fn history(&self) -> &[u16] {
        &self.hist[..self.hlen as usize]
    }
}

struct Search<'a, D> {
    model: &'a Model,
    lexicon: Option<&'a Fst<D>>,
    keep: usize,
    /// (parent, unit) for every step taken; a hypothesis points at its last step.
    steps: Vec<(u32, Unit)>,
}

impl<D: AsRef<[u8]>> Search<'_, D> {
    fn extend(&mut self, h: &Hyp, id: u16) -> Option<Hyp> {
        let g = self.model.graphones[id as usize];
        if !may_follow((h.last != 0).then_some(h.last), g.unit) {
            return None;
        }
        let (mut node, mut out) = (h.node, h.out);
        if let Some(fst) = self.lexicon {
            let n = fst.node(node);
            let t = n.transition(n.find_input(g.unit)?);
            node = t.addr;
            out += t.out.value();
        }
        let score = h.score + self.model.lm.logp(h.history(), id);
        if score == f32::NEG_INFINITY {
            return None;
        }
        let mut next = Hyp {
            score,
            last: g.unit,
            node,
            out,
            back: self.steps.len() as u32,
            ..*h
        };
        if (next.hlen as usize) < self.keep {
            next.hlen += 1;
        } else if self.keep > 0 {
            next.hist.copy_within(1..self.keep, 0);
        }
        if self.keep > 0 {
            next.hist[next.hlen as usize - 1] = id;
        }
        self.steps.push((h.back, g.unit));
        Some(next)
    }

    fn units(&self, mut back: u32) -> Vec<Unit> {
        let mut out = Vec::new();
        while back != u32::MAX {
            let (parent, unit) = self.steps[back as usize];
            out.push(unit);
            back = parent;
        }
        out.reverse();
        out
    }
}

/// Keeps the best `beam` hypotheses, and of those with the same future only the best.
fn prune(hyps: &mut Vec<Hyp>, beam: usize) {
    hyps.sort_unstable_by(|a, b| b.score.total_cmp(&a.score));
    let mut seen = HashMap::with_capacity(hyps.len());
    hyps.retain(|h| seen.insert((h.hist, h.hlen, h.last, h.node), ()).is_none());
    hyps.truncate(beam);
}

/// The words that best explain `letters` (lowercase a to z), best first, at most 1 entry per word.
///
/// With `complete`, the best partial readings in the lexicon also give the
/// commonest lexicon word they start, scored without the end of the word.
pub fn search<D: AsRef<[u8]>>(
    model: &Model,
    letters: &[u8],
    lexicon: Option<&Fst<D>>,
    complete: Option<&Completions>,
    beam: usize,
) -> Vec<Found> {
    let n = letters.len();
    if n == 0 || beam == 0 {
        return Vec::new();
    }
    let keep = (model.lm.order() - 1).min(HIST);
    let mut s = Search {
        model,
        lexicon,
        keep,
        steps: Vec::new(),
    };
    let start = Hyp {
        score: 0.0,
        hist: [0; HIST],
        hlen: u8::from(keep > 0),
        last: 0,
        node: lexicon.map_or(NO_NODE, |f| f.root().addr()),
        out: 0,
        back: u32::MAX,
    };
    let mut beams: Vec<Vec<Hyp>> = vec![Vec::new(); n + 1];
    beams[0].push(start);
    let empty: &[u16] = model.graphones_for(b"");
    for i in 0..=n {
        let mut here = std::mem::take(&mut beams[i]);
        prune(&mut here, beam);
        // 1 round only, so at most 1 unit in a row takes no letters.
        let mut added = Vec::new();
        for h in &here {
            added.extend(empty.iter().filter_map(|&id| s.extend(h, id)));
        }
        here.extend(added);
        prune(&mut here, beam);
        if i == n {
            beams[n] = here;
            break;
        }
        for h in &here {
            for len in 1..=MAX_CHUNK.min(n - i) {
                for &id in model.graphones_for(&letters[i..i + len]) {
                    if let Some(next) = s.extend(h, id) {
                        beams[i + len].push(next);
                    }
                }
            }
        }
    }
    let mut best: HashMap<Vec<Unit>, Found> = HashMap::new();
    for h in &beams[n] {
        if h.last == 0 || !may_end(h.last) {
            continue;
        }
        let value = match lexicon {
            Some(fst) => {
                let node = fst.node(h.node);
                if !node.is_final() {
                    continue;
                }
                Some(h.out + node.final_output().value())
            }
            None => None,
        };
        let lm = h.score + model.lm.logp(h.history(), 0);
        let units = s.units(h.back);
        if best.get(&units).is_none_or(|f| f.extra > 0 || f.lm < lm) {
            best.insert(
                units.clone(),
                Found {
                    units,
                    lm,
                    value,
                    extra: 0,
                },
            );
        }
    }
    if let (Some(fst), Some(c)) = (lexicon, complete) {
        for h in beams[n].iter().filter(|h| h.last != 0).take(COMPLETE) {
            let prefix = s.units(h.back);
            let node = fst.node(h.node);
            // The best word from here, and every word 1 unit longer.
            let mut words: Vec<(Vec<u8>, u64)> = c.best(fst, h.node).into_iter().collect();
            for t in node.transitions() {
                let next = fst.node(t.addr);
                if next.is_final() {
                    words.push((vec![t.inp], t.out.value() + next.final_output().value()));
                }
            }
            for (rest, total) in words {
                if rest.is_empty() {
                    continue;
                }
                let mut units = prefix.clone();
                units.extend_from_slice(&rest);
                let keep = best
                    .get(&units)
                    .is_none_or(|f| f.extra > 0 && f.lm < h.score);
                if keep {
                    let found = Found {
                        lm: h.score,
                        value: Some(h.out + total),
                        extra: rest.len(),
                        units: units.clone(),
                    };
                    best.insert(units, found);
                }
            }
        }
    }
    let mut out: Vec<Found> = best.into_values().collect();
    out.sort_by(|a, b| b.lm.total_cmp(&a.lm).then_with(|| a.units.cmp(&b.units)));
    out
}
