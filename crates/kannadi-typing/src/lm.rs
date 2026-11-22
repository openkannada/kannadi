//! A backoff n-gram model over graphones, stored as a trie with 8-bit
//! quantized log probabilities.
//!
//! Token 0 marks a word boundary: it is the first context of every word and the
//! last token predicted. Level `k` holds the n-grams of length `k + 1`, sorted,
//! so the children of an entry are a contiguous range of the next level.

use crate::bytes::{Reader, Writer};

/// The longest history the model can use.
pub const MAX_ORDER: usize = 8;

/// A uniform 8-bit quantizer for natural-log values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quant {
    pub min: f32,
    pub step: f32,
}

impl Quant {
    /// A quantizer covering `min..=max`.
    #[cfg(any(test, feature = "train"))]
    pub fn new(min: f32, max: f32) -> Quant {
        let step = ((max - min) / 255.0).max(f32::EPSILON);
        Quant { min, step }
    }
    #[cfg(any(test, feature = "train"))]
    pub fn encode(&self, x: f32) -> u8 {
        ((x - self.min) / self.step).round().clamp(0.0, 255.0) as u8
    }
    pub fn decode(&self, q: u8) -> f32 {
        self.min + self.step * q as f32
    }
}

/// One level of the trie.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Level {
    /// The last token of each n-gram; empty on level 0, whose index is the token.
    pub ids: Vec<u16>,
    pub prob: Vec<u8>,
    /// Backoff weight of each n-gram as a context; empty on the top level.
    pub bow: Vec<u8>,
    /// Children of entry `i` are `child[i]..child[i + 1]` of the next level; empty on the top level.
    pub child: Vec<u32>,
}

/// The model.
#[derive(Clone, Debug, PartialEq)]
pub struct Lm {
    pub levels: Vec<Level>,
    pub prob_q: Quant,
    pub bow_q: Quant,
}

impl Lm {
    /// The n-gram order.
    pub fn order(&self) -> usize {
        self.levels.len()
    }

    /// The index of `ngram` on level `ngram.len() - 1`.
    fn find(&self, ngram: &[u16]) -> Option<usize> {
        let (&first, rest) = ngram.split_first()?;
        let mut idx = first as usize;
        if idx >= self.levels[0].prob.len() {
            return None;
        }
        for (k, &id) in rest.iter().enumerate() {
            idx = self.child(k, idx, id)?;
        }
        Some(idx)
    }

    /// The child of entry `idx` on level `k` whose token is `id`.
    fn child(&self, k: usize, idx: usize, id: u16) -> Option<usize> {
        let parent = &self.levels[k];
        let next = self.levels.get(k + 1)?;
        let (lo, hi) = (parent.child[idx] as usize, parent.child[idx + 1] as usize);
        next.ids[lo..hi].binary_search(&id).ok().map(|i| lo + i)
    }

    /// The natural log of P(`token` | `history`), where `history` ends with the
    /// newest token. Only the last `order - 1` tokens of the history are used.
    pub fn logp(&self, history: &[u16], token: u16) -> f32 {
        let keep = history.len().min(self.order() - 1);
        let history = &history[history.len() - keep..];
        let mut backoff = 0.0;
        for start in 0..history.len() {
            let ctx = &history[start..];
            let Some(c) = self.find(ctx) else { continue };
            let k = ctx.len() - 1;
            if let Some(i) = self.child(k, c, token) {
                return backoff + self.prob_q.decode(self.levels[k + 1].prob[i]);
            }
            backoff += self.bow_q.decode(self.levels[k].bow[c]);
        }
        match self.levels[0].prob.get(token as usize) {
            Some(&q) => backoff + self.prob_q.decode(q),
            None => f32::NEG_INFINITY,
        }
    }

    pub(crate) fn write(&self, w: &mut Writer) {
        w.u32(self.levels.len() as u32);
        for q in [self.prob_q, self.bow_q] {
            w.f32(q.min);
            w.f32(q.step);
        }
        for level in &self.levels {
            w.u16s(&level.ids);
            w.bytes(&level.prob);
            w.bytes(&level.bow);
            w.u32s(&level.child);
        }
    }

    pub(crate) fn read(r: &mut Reader) -> Option<Lm> {
        let order = r.u32()? as usize;
        if order == 0 || order > MAX_ORDER {
            return None;
        }
        let prob_q = Quant {
            min: r.f32()?,
            step: r.f32()?,
        };
        let bow_q = Quant {
            min: r.f32()?,
            step: r.f32()?,
        };
        let mut levels = Vec::with_capacity(order);
        for _ in 0..order {
            levels.push(Level {
                ids: r.u16s()?,
                prob: r.bytes()?.to_vec(),
                bow: r.bytes()?.to_vec(),
                child: r.u32s()?,
            });
        }
        let lm = Lm {
            levels,
            prob_q,
            bow_q,
        };
        lm.is_consistent().then_some(lm)
    }

    /// Whether every array has the length and offsets the lookups rely on.
    fn is_consistent(&self) -> bool {
        let n = self.levels.len();
        self.levels.iter().enumerate().all(|(k, l)| {
            let len = l.prob.len();
            let ids_ok = if k == 0 {
                l.ids.is_empty()
            } else {
                l.ids.len() == len
            };
            if k + 1 == n {
                return ids_ok && l.bow.is_empty() && l.child.is_empty();
            }
            let next = self.levels[k + 1].prob.len();
            ids_ok
                && l.bow.len() == len
                && l.child.len() == len + 1
                && l.child.windows(2).all(|w| w[0] <= w[1])
                && l.child.last().is_some_and(|&e| e as usize == next)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tokens 0, 1, 2; the bigram 1 2 is stored.
    fn tiny() -> Lm {
        let q = Quant::new(-10.0, 0.0);
        Lm {
            levels: vec![
                Level {
                    ids: vec![],
                    prob: vec![q.encode(-1.0), q.encode(-2.0), q.encode(-3.0)],
                    bow: vec![q.encode(0.0), q.encode(-0.5), q.encode(0.0)],
                    child: vec![0, 0, 1, 1],
                },
                Level {
                    ids: vec![2],
                    prob: vec![q.encode(-0.2)],
                    bow: vec![],
                    child: vec![],
                },
            ],
            prob_q: q,
            bow_q: q,
        }
    }

    #[test]
    fn uses_the_stored_ngram_or_backs_off() {
        let lm = tiny();
        assert!((lm.logp(&[1], 2) - -0.2).abs() < 0.05);
        assert!((lm.logp(&[1], 0) - (-0.5 - 1.0)).abs() < 0.05);
        assert!((lm.logp(&[2], 1) - -2.0).abs() < 0.05);
        assert_eq!(lm.logp(&[], 9), f32::NEG_INFINITY);
    }

    #[test]
    fn round_trips_and_rejects_broken_offsets() {
        let lm = tiny();
        let mut w = Writer::default();
        lm.write(&mut w);
        let bytes = w.finish();
        assert_eq!(Lm::read(&mut Reader::new(&bytes)), Some(lm.clone()));
        let mut broken = lm;
        broken.levels[0].child[3] = 7;
        let mut w = Writer::default();
        broken.write(&mut w);
        assert_eq!(Lm::read(&mut Reader::new(&w.finish())), None);
    }
}
