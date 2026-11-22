//! Many-to-1 alignment of Latin letters to Kannada units, learned with EM.
//!
//! Each unit takes 0 to 3 letters. Only the inherent a and the signs such as
//! the anusvara may take none, since typists drop them ("kannda", "samshodhane"
//! for ಸಂಶೋಧನೆ written as "sashodhane"). The model is a joint distribution over
//! (chunk, unit) pairs; each iteration counts every alignment of every pair,
//! weighted by its probability (forward-backward).

use crate::model::Graphone;
use crate::roman::{chunk_code, CHUNK_CODES, MAX_CHUNK};
use crate::units::{kind, Kind, Unit, COUNT};

/// The chunk lengths a unit may take.
fn lengths(unit: Unit) -> std::ops::RangeInclusive<usize> {
    match kind(unit) {
        Kind::Inherent | Kind::Mark => 0..=2,
        _ => 1..=MAX_CHUNK,
    }
}

fn slot(chunk: &[u8], unit: Unit) -> usize {
    chunk_code(chunk) as usize * COUNT + unit as usize
}

/// A training pair: Latin letters and the units of the Kannada word.
pub type Pair = (Vec<u8>, Vec<Unit>);

/// The joint probability of each (chunk, unit) pair.
pub struct Aligner {
    p: Vec<f64>,
}

impl Aligner {
    /// Starts with every allowed pair equally likely.
    pub fn uniform() -> Aligner {
        Aligner {
            p: vec![1.0; CHUNK_CODES * COUNT],
        }
    }

    /// Runs `iterations` rounds of EM over `pairs` on `threads` threads.
    pub fn train(pairs: &[Pair], iterations: usize, threads: usize) -> Aligner {
        let mut a = Aligner::uniform();
        for _ in 0..iterations {
            let size = pairs.len().div_ceil(threads.max(1)).max(1);
            let counts: Vec<Vec<f64>> = std::thread::scope(|s| {
                let a = &a;
                let jobs: Vec<_> = pairs
                    .chunks(size)
                    .map(|part| {
                        s.spawn(move || {
                            let mut c = vec![0.0; CHUNK_CODES * COUNT];
                            part.iter().for_each(|(r, u)| a.expect(r, u, &mut c));
                            c
                        })
                    })
                    .collect();
                jobs.into_iter()
                    .map(|j| j.join().expect("EM worker"))
                    .collect()
            });
            let mut total = vec![0.0; CHUNK_CODES * COUNT];
            for c in counts {
                total.iter_mut().zip(c).for_each(|(t, x)| *t += x);
            }
            let sum: f64 = total.iter().sum();
            a.p = total.into_iter().map(|x| x / sum).collect();
        }
        a
    }

    fn weight(&self, roman: &[u8], i: usize, len: usize, unit: Unit) -> f64 {
        self.p[slot(&roman[i..i + len], unit)]
    }

    /// Adds the expected count of each (chunk, unit) in the alignments of 1 pair.
    fn expect(&self, roman: &[u8], units: &[Unit], counts: &mut [f64]) {
        let (n, m) = (roman.len(), units.len());
        let at = |i: usize, j: usize| i * (m + 1) + j;
        let mut alpha = vec![0.0; (n + 1) * (m + 1)];
        let mut beta = vec![0.0; (n + 1) * (m + 1)];
        alpha[at(0, 0)] = 1.0;
        for j in 0..m {
            for i in 0..=n {
                let a = alpha[at(i, j)];
                if a == 0.0 {
                    continue;
                }
                for len in lengths(units[j]).filter(|&l| i + l <= n) {
                    alpha[at(i + len, j + 1)] += a * self.weight(roman, i, len, units[j]);
                }
            }
        }
        let z = alpha[at(n, m)];
        if z <= 0.0 || !z.is_finite() {
            return;
        }
        beta[at(n, m)] = 1.0;
        for j in (0..m).rev() {
            for i in (0..=n).rev() {
                let mut b = 0.0;
                for len in lengths(units[j]).filter(|&l| i + l <= n) {
                    b += self.weight(roman, i, len, units[j]) * beta[at(i + len, j + 1)];
                }
                beta[at(i, j)] = b;
            }
        }
        for j in 0..m {
            for i in 0..=n {
                let a = alpha[at(i, j)];
                if a == 0.0 {
                    continue;
                }
                for len in lengths(units[j]).filter(|&l| i + l <= n) {
                    let w = self.weight(roman, i, len, units[j]);
                    counts[slot(&roman[i..i + len], units[j])] +=
                        a * w * beta[at(i + len, j + 1)] / z;
                }
            }
        }
    }

    /// The likeliest alignment of a pair and its natural-log probability, or
    /// `None` when the letters cannot be split among the units.
    pub fn best(&self, roman: &[u8], units: &[Unit]) -> Option<(Vec<Graphone>, f64)> {
        let (n, m) = (roman.len(), units.len());
        let at = |i: usize, j: usize| i * (m + 1) + j;
        let mut score = vec![f64::NEG_INFINITY; (n + 1) * (m + 1)];
        let mut from = vec![0usize; (n + 1) * (m + 1)];
        score[at(0, 0)] = 0.0;
        for j in 0..m {
            for i in 0..=n {
                let s = score[at(i, j)];
                if s == f64::NEG_INFINITY {
                    continue;
                }
                for len in lengths(units[j]).filter(|&l| i + l <= n) {
                    let w = self.weight(roman, i, len, units[j]);
                    let t = s + w.ln();
                    if w > 0.0 && t > score[at(i + len, j + 1)] {
                        score[at(i + len, j + 1)] = t;
                        from[at(i + len, j + 1)] = len;
                    }
                }
            }
        }
        let total = score[at(n, m)];
        if total == f64::NEG_INFINITY {
            return None;
        }
        let mut out = Vec::with_capacity(m);
        let mut i = n;
        for j in (1..=m).rev() {
            let len = from[at(i, j)];
            out.push(Graphone {
                chunk: chunk_code(&roman[i - len..i]),
                unit: units[j - 1],
            });
            i -= len;
        }
        out.reverse();
        Some((out, total))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roman::chunk_text;
    use crate::units::units;

    #[test]
    fn learns_to_split_kannada() {
        let words = [
            ("kannada", "ಕನ್ನಡ"),
            ("kannda", "ಕನ್ನಡ"),
            ("nadu", "ನಾಡು"),
            ("naadu", "ನಾಡು"),
            ("kadu", "ಕಾಡು"),
            ("kaadu", "ಕಾಡು"),
            ("kodu", "ಕೊಡು"),
            ("nanna", "ನನ್ನ"),
            ("dana", "ದನ"),
            ("kana", "ಕಾಣ"),
            ("na", "ನ"),
            ("ka", "ಕ"),
            ("du", "ದು"),
            ("nu", "ನು"),
            ("ku", "ಕು"),
            ("maadi", "ಮಾಡಿ"),
            ("baa", "ಬಾ"),
        ];
        let pairs: Vec<Pair> = words
            .iter()
            .map(|(r, k)| (r.as_bytes().to_vec(), units(k).unwrap()))
            .collect();
        let a = Aligner::train(&pairs, 5, 2);
        let (g, _) = a.best(b"kannada", &units("ಕನ್ನಡ").unwrap()).unwrap();
        let chunks: Vec<String> = g.iter().map(|g| chunk_text(g.chunk)).collect();
        assert_eq!(chunks, ["k", "a", "n", "n", "a", "d", "a"]);
        let (g, _) = a.best(b"naadu", &units("ನಾಡು").unwrap()).unwrap();
        let chunks: Vec<String> = g.iter().map(|g| chunk_text(g.chunk)).collect();
        assert_eq!(chunks, ["n", "aa", "d", "u"]);
        assert!(a.best(b"k", &units("ಕನ್ನಡ").unwrap()).is_none());
    }
}
