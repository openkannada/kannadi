//! Interpolated modified Kneser-Ney estimation, written out as a backoff trie.
//!
//! Lower orders use continuation counts (how many different tokens precede an
//! n-gram), except n-grams that start at the word boundary. Pruned n-grams give
//! their probability mass to the backoff weight of their context, so every
//! distribution still sums to 1.

use crate::lm::{Level, Lm, Quant, MAX_ORDER};
use std::collections::HashMap;

type Key = u128;

fn key(ngram: &[u16]) -> Key {
    ngram.iter().enumerate().fold(0, |k, (i, &id)| {
        k | (id as Key) << (16 * (MAX_ORDER - 1 - i))
    })
}

fn slot(k: Key, i: usize) -> u16 {
    (k >> (16 * (MAX_ORDER - 1 - i))) as u16
}

fn ids(k: Key, n: usize) -> Vec<u16> {
    (0..n).map(|i| slot(k, i)).collect()
}

/// The key of the first `n` tokens of `k`.
fn prefix(k: Key, n: usize) -> Key {
    if n == 0 {
        0
    } else {
        k & !(Key::MAX >> (16 * n))
    }
}

/// Settings for [`estimate`].
pub struct Settings {
    pub order: usize,
    /// For each order from 1, the smallest count an n-gram needs to be kept.
    pub min_count: Vec<u32>,
}

/// Estimates a model from sequences of token ids, where 0 is reserved for the
/// word boundary and `vocab` is 1 more than the largest id.
pub fn estimate(seqs: &[Vec<u16>], vocab: usize, settings: &Settings) -> Lm {
    let order = settings.order.clamp(1, MAX_ORDER);
    let mut raw: Vec<HashMap<Key, u32>> = vec![HashMap::new(); order + 1];
    let mut padded = Vec::new();
    for s in seqs {
        padded.clear();
        padded.push(0);
        padded.extend_from_slice(s);
        padded.push(0);
        for t in 1..padded.len() {
            for n in 1..=order.min(t + 1) {
                *raw[n].entry(key(&padded[t + 1 - n..=t])).or_default() += 1;
            }
        }
    }
    // Adjusted counts: continuation counts below the top order.
    let mut adj: Vec<HashMap<Key, u32>> = vec![HashMap::new(); order + 1];
    adj[order] = std::mem::take(&mut raw[order]);
    for n in (1..order).rev() {
        let mut cont: HashMap<Key, u32> = HashMap::new();
        let longer = if n + 1 == order {
            &adj[order]
        } else {
            &raw[n + 1]
        };
        for &k in longer.keys() {
            *cont.entry(k << 16).or_default() += 1;
        }
        // An n-gram that starts at the boundary has nothing before it: keep its own count.
        for (&k, &c) in &raw[n] {
            if slot(k, 0) == 0 {
                cont.insert(k, c);
            }
        }
        adj[n] = cont;
    }
    let mut prob: Vec<HashMap<Key, f64>> = vec![HashMap::new(); order + 1];
    let mut bow: Vec<HashMap<Key, f64>> = vec![HashMap::new(); order + 1];
    let lower = |prob: &[HashMap<Key, f64>], bow: &[HashMap<Key, f64>], ngram: &[u16]| -> f64 {
        let mut b = 1.0;
        for start in 1..ngram.len() {
            let tail = &ngram[start..];
            if let Some(&p) = prob[tail.len()].get(&key(tail)) {
                return b * p;
            }
            b *= bow[tail.len() - 1]
                .get(&key(&tail[..tail.len() - 1]))
                .copied()
                .unwrap_or(1.0);
        }
        b / vocab as f64
    };
    for n in 1..=order {
        let d = discounts(&adj[n]);
        let mut entries: Vec<(Key, u32)> = adj[n].iter().map(|(&k, &c)| (k, c)).collect();
        entries.sort_unstable();
        let min = settings.min_count.get(n - 1).copied().unwrap_or(1).max(1);
        let mut i = 0;
        while i < entries.len() {
            let ctx = prefix(entries[i].0, n - 1);
            let mut j = i;
            while j < entries.len() && prefix(entries[j].0, n - 1) == ctx {
                j += 1;
            }
            let group = &entries[i..j];
            let total: f64 = group.iter().map(|&(_, c)| c as f64).sum();
            let context_kept = n <= 2 || prob[n - 1].contains_key(&ctx);
            let kept: Vec<(Key, f64)> = group
                .iter()
                .filter(|&&(_, c)| context_kept && (n == 1 || c >= min))
                .map(|&(k, c)| (k, (c as f64 - d[(c as usize).min(3)]).max(0.0) / total))
                .collect();
            let gamma = (1.0 - kept.iter().map(|&(_, p)| p).sum::<f64>()).max(1e-12);
            for (k, p) in kept {
                let full = p + gamma * lower(&prob, &bow, &ids(k, n));
                prob[n].insert(k, full);
            }
            if n > 1 {
                bow[n - 1].insert(ctx, gamma);
            } else {
                bow[0].insert(0, gamma);
            }
            i = j;
        }
        if n == 1 {
            // Every token gets a unigram, even one never seen.
            let gamma = bow[0][&0];
            for id in 0..vocab as u16 {
                prob[1].entry(key(&[id])).or_insert(gamma / vocab as f64);
            }
        }
    }
    build_trie(&prob, &bow, order, vocab)
}

/// D1, D2 and D3+ from the counts of counts (index 0 is unused).
fn discounts(counts: &HashMap<Key, u32>) -> [f64; 4] {
    let mut n = [0f64; 5];
    for &c in counts.values() {
        if (1..=4).contains(&c) {
            n[c as usize] += 1.0;
        }
    }
    if n[1] == 0.0 || n[2] == 0.0 || n[3] == 0.0 {
        return [0.0, 0.5, 1.0, 1.5];
    }
    let y = n[1] / (n[1] + 2.0 * n[2]);
    let d1 = (1.0 - 2.0 * y * n[2] / n[1]).clamp(0.0, 1.0);
    let d2 = (2.0 - 3.0 * y * n[3] / n[2]).clamp(0.0, 2.0);
    let d3 = if n[4] > 0.0 {
        (3.0 - 4.0 * y * n[4] / n[3]).clamp(0.0, 3.0)
    } else {
        1.5
    };
    [0.0, d1, d2, d3]
}

fn build_trie(
    prob: &[HashMap<Key, f64>],
    bow: &[HashMap<Key, f64>],
    order: usize,
    vocab: usize,
) -> Lm {
    let mut sorted: Vec<Vec<Key>> = (0..=order)
        .map(|n| prob[n].keys().copied().collect())
        .collect();
    sorted.iter_mut().for_each(|v| v.sort_unstable());
    let lp = |p: f64| (p.max(1e-30)).ln() as f32;
    let all_p = (1..=order).flat_map(|n| prob[n].values().map(|&p| lp(p)));
    let pmin = all_p.fold(0f32, f32::min);
    let bmin = bow
        .iter()
        .flat_map(|m| m.values().map(|&b| lp(b)))
        .fold(0f32, f32::min);
    let prob_q = Quant::new(pmin, 0.0);
    let bow_q = Quant::new(bmin, 0.0);
    let mut levels = Vec::with_capacity(order);
    for n in 1..=order {
        let keys = &sorted[n];
        let mut level = Level::default();
        if n == 1 {
            debug_assert_eq!(keys.len(), vocab);
        } else {
            level.ids = keys.iter().map(|&k| slot(k, n - 1)).collect();
        }
        level.prob = keys.iter().map(|k| prob_q.encode(lp(prob[n][k]))).collect();
        if n < order {
            level.bow = keys
                .iter()
                .map(|k| bow_q.encode(lp(bow[n].get(k).copied().unwrap_or(1.0))))
                .collect();
            let next = &sorted[n + 1];
            let mut j = 0;
            level.child.push(0);
            for &k in keys {
                while j < next.len() && prefix(next[j], n) == k {
                    j += 1;
                }
                level.child.push(j as u32);
            }
            debug_assert_eq!(j, next.len());
        }
        levels.push(level);
    }
    Lm {
        levels,
        prob_q,
        bow_q,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn total(lm: &Lm, history: &[u16], vocab: u16) -> f64 {
        (0..vocab).map(|t| (lm.logp(history, t) as f64).exp()).sum()
    }

    #[test]
    fn distributions_sum_to_1() {
        let seqs: Vec<Vec<u16>> = vec![
            vec![1, 2, 3],
            vec![1, 2, 4],
            vec![2, 3],
            vec![1, 3, 3, 2],
            vec![4],
        ];
        for min in [1, 2] {
            let lm = estimate(
                &seqs,
                5,
                &Settings {
                    order: 3,
                    min_count: vec![1, min, min],
                },
            );
            for history in [&[0u16][..], &[0, 1], &[1, 2], &[3, 3], &[4, 4], &[]] {
                let s = total(&lm, history, 5);
                assert!((s - 1.0).abs() < 0.08, "history {history:?} sums to {s}");
            }
        }
    }

    #[test]
    fn prefers_what_it_saw() {
        let seqs: Vec<Vec<u16>> = (0..20).map(|_| vec![1, 2]).chain([vec![1, 3]]).collect();
        let lm = estimate(
            &seqs,
            4,
            &Settings {
                order: 2,
                min_count: vec![1, 1],
            },
        );
        assert!(lm.logp(&[1], 2) > lm.logp(&[1], 3));
        assert!(lm.logp(&[0], 1) > lm.logp(&[0], 2));
    }
}
