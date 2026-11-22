//! Type Kannada in English letters.
//!
//! [`Engine::suggest`] turns loosely typed Latin letters into ranked Kannada
//! words: "kannada", "kannda" and "Kannada" all give ಕನ್ನಡ first. Candidates
//! come from 3 places:
//!
//! - a beam search through a lexicon of 232,003 Kannada words, scored by
//!   an n-gram model over (letters, Kannada unit) pairs trained on the
//!   Aksharantar transliteration pairs;
//! - the same search with no lexicon, for words the lexicon lacks;
//! - the strict Baraha and ITRANS readings of the input, which win when the
//!   input is typed precisely: capitals inside a word, such as `kannaDa`, or a
//!   mark, such as the dot in `gaa.ndhii`.
//!
//! Load only model files you trust: a damaged file is rejected, but a file
//! crafted to break the lexicon automaton can still make the engine panic.
//!
//! The ranking adds the word's frequency in Kannada Wikipedia, how often it
//! follows the previous word, and what the user picked before.
//! [`Engine::learn`] records a pick in memory only; [`Engine::export_user`] and
//! [`Engine::import_user`] let the caller keep picks across sessions.
//!
//! ```
//! let engine = kannadi_typing::Engine::bundled();
//! assert_eq!(engine.suggest("kannada", None, 3)[0], "ಕನ್ನಡ");
//! ```

mod bytes;
mod complete;
mod decode;
mod lm;
mod model;
mod roman;
mod rules;
#[cfg(feature = "train")]
pub mod train;
pub mod units;
mod user;

use model::class_ln;
pub use model::{BadModel, Model, Weights};

use fst::raw::Fst;
use std::collections::HashMap;
use user::User;

/// A Kannada word for the input, with the evidence for it.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub word: String,
    /// Natural log of P(letters, word) under the n-gram model.
    pub lm: f32,
    /// Natural log of 1 plus the word's count in the corpus; 0 when not in the lexicon.
    pub freq: f32,
    /// Whether the lexicon has the word, or the user picked it before.
    pub known: bool,
    /// How much likelier the previous word makes this one, as a natural log.
    pub lift: f32,
    /// Whether the strict Baraha reading of the input gives this word.
    pub rule: bool,
    /// Whether the input is typed precisely: capitals inside the word (kannaDa) or a Baraha or ITRANS mark.
    pub precise: bool,
    /// Natural log of 1 plus how often the user picked the word.
    pub user: f32,
    /// For a word picked before for exactly these letters, how recent the pick is (higher is newer).
    pub picked: Option<usize>,
    /// Units added after the letters ran out, for a lexicon word the letters only start; 0 for a whole word.
    pub extra: usize,
}

impl Candidate {
    /// The candidate's score under `w`; higher is better.
    pub fn score(&self, w: &Weights) -> f32 {
        let mut s = self.lm
            + w.freq * self.freq
            + w.pair * self.lift
            + w.user * self.user
            + w.extend * self.extra as f32;
        if self.known {
            s += w.known;
        }
        if self.rule {
            s += w.rule;
            if self.precise {
                s += w.precise;
            }
        }
        s
    }
}

/// Up to this many letters, the strict reading comes first: the input is most
/// likely the start of a longer word, and the first word is shown as it is typed.
const SHORT: usize = 2;
/// The strict reading is never further down than this place (0-based).
const STRICT_BY: usize = 4;

/// Sorts the candidates for an input of `typed` letters best first under `w`.
///
/// Words picked before for the same letters come first, the newest pick first.
/// Otherwise the first word is a reading of the letters as a whole word, never
/// a completion:
///
/// - for 1 or 2 typed letters, the strict reading, with the best whole word second;
/// - when a completion outscores every whole word, the letters are likely the
///   start of a longer word, so the whole word that best fits the letters and
///   the corpus, without the bonus for being in the lexicon;
/// - otherwise the best-scoring whole word.
///
/// The strict reading always stays in the first 5, after any picks.
pub fn rank(candidates: &mut [Candidate], w: &Weights, typed: usize) {
    candidates.sort_by(|a, b| {
        b.picked
            .cmp(&a.picked)
            .then_with(|| b.score(w).total_cmp(&a.score(w)))
            .then_with(|| a.word.cmp(&b.word))
    });
    let whole = |c: &Candidate| c.extra == 0;
    let strict = |c: &Candidate| c.rule && c.extra == 0;
    let picks = candidates.iter().take_while(|c| c.picked.is_some()).count();
    let first_is_completion = candidates.first().is_some_and(|c| c.extra > 0);
    let short = typed <= SHORT && candidates.iter().any(strict);
    let lead = if picks > 0 {
        None
    } else if short {
        candidates.iter().position(strict)
    } else if first_is_completion {
        let fit = |c: &Candidate| c.score(w) - if c.known { w.known } else { 0.0 };
        let mut best: Option<(usize, f32)> = None;
        for (i, c) in candidates.iter().enumerate().filter(|(_, c)| whole(c)) {
            if best.is_none_or(|(_, f)| fit(c) > f) {
                best = Some((i, fit(c)));
            }
        }
        best.map(|(i, _)| i)
    } else {
        None
    };
    if let Some(i) = lead {
        candidates[..=i].rotate_right(1);
        if short {
            if let Some(j) = candidates.iter().skip(1).position(whole).map(|j| j + 1) {
                candidates[1..=j].rotate_right(1);
            }
        }
    }
    let by = STRICT_BY.max(picks);
    if let Some(i) = candidates.iter().position(strict) {
        if i > by {
            candidates[by..=i].rotate_right(1);
        }
    }
}

/// The typing engine: a model and 1 user's picks.
pub struct Engine {
    model: Model,
    user: User,
}

#[cfg(feature = "bundled")]
static BUNDLED: &[u8] = include_bytes!("../model/kn.typing");

impl Engine {
    /// An engine over `model`, with no picks.
    pub fn new(model: Model) -> Engine {
        Engine {
            model,
            user: User::default(),
        }
    }

    /// An engine over the model compiled into the crate.
    #[cfg(feature = "bundled")]
    pub fn bundled() -> Engine {
        Engine::new(
            Model::from_bytes(BUNDLED).expect("the bundled model is valid; a test checks it"),
        )
    }

    /// The model.
    pub fn model(&self) -> &Model {
        &self.model
    }

    /// Replaces the ranking weights. Returns false, and keeps the old ones, if
    /// they are not [valid](Weights::is_valid).
    pub fn set_weights(&mut self, weights: Weights) -> bool {
        weights.is_valid() && {
            self.model.weights = weights;
            true
        }
    }

    fn lexicon_class(&self, units: &[u8]) -> Option<u64> {
        self.model.lexicon.get(units).map(|o| o.value())
    }

    /// Every candidate for `input` with its evidence, best first. `prev` is the
    /// Kannada word before the cursor, if any. Input with anything other than
    /// ASCII letters and the marks `.`, `~` and `^` gives no candidates.
    pub fn candidates(&self, input: &str, prev: Option<&str>) -> Vec<Candidate> {
        let Some(letters) = roman::letters(input) else {
            return Vec::new();
        };
        if letters.is_empty() {
            return Vec::new();
        }
        let w = &self.model.weights;
        let precise = roman::is_precise(input);
        // Per word: model score, lexicon class, units added by completion.
        let mut found: HashMap<Vec<u8>, (f32, Option<u64>, usize)> = HashMap::new();
        let mut add = |units: Vec<u8>, lm: f32, class: Option<u64>, extra: usize| {
            let e = found.entry(units).or_insert((lm, class, extra));
            if (extra, -lm) < (e.2, -e.0) {
                (e.0, e.2) = (lm, extra);
            }
            e.1 = e.1.or(class);
        };
        let m = &self.model;
        for f in decode::search(
            m,
            &letters,
            Some(&m.lexicon),
            Some(&m.completions),
            w.beam as usize,
        ) {
            add(f.units, f.lm, f.value, f.extra);
        }
        for f in decode::search::<Vec<u8>>(m, &letters, None, None, w.open_beam as usize) {
            let class = self.lexicon_class(&f.units);
            add(f.units, f.lm, class, 0);
        }
        if let Some(extra) = self.user.extra() {
            for f in decode::search(m, &letters, Some(extra.as_fst()), None, 16) {
                let class = self.lexicon_class(&f.units);
                add(f.units, f.lm, class, 0);
            }
        }
        let rules = rules::readings(input);
        let mut picked: Vec<(Vec<u8>, usize)> = Vec::new();
        for (rank, word) in self.user.picks_for(input) {
            if let Some(u) = units::units(word) {
                picked.push((u, rank));
            }
        }
        let floor = found.values().map(|f| f.0).fold(0f32, f32::min) - 10.0;
        for u in rules.iter().chain(picked.iter().map(|(u, _)| u)) {
            if !found.contains_key(u) {
                let lm = self.forced(&letters, u).unwrap_or(floor);
                found.insert(u.clone(), (lm, self.lexicon_class(u), 0));
            }
        }
        let prev = prev.and_then(units::units).map(|u| units::text(&u));
        let mut out: Vec<Candidate> = found
            .into_iter()
            .map(|(u, (lm, class, extra))| {
                let word = units::text(&u);
                let picks = self.user.count(&word);
                Candidate {
                    lift: prev
                        .as_deref()
                        .map_or(0.0, |p| self.model.pairs.lift(p, &word)),
                    user: (picks as f32).ln_1p(),
                    picked: picked
                        .iter()
                        .filter(|(p, _)| *p == u)
                        .map(|&(_, r)| r)
                        .max(),
                    rule: rules.contains(&u),
                    word,
                    lm,
                    freq: class.map_or(0.0, class_ln),
                    known: class.is_some() || picks > 0,
                    precise,
                    extra,
                }
            })
            .collect();
        rank(&mut out, w, roman::typed_letters(input));
        out
    }

    /// The best `n` Kannada words for `input`, best first.
    pub fn suggest(&self, input: &str, prev: Option<&str>, n: usize) -> Vec<String> {
        self.candidates(input, prev)
            .into_iter()
            .take(n)
            .map(|c| c.word)
            .collect()
    }

    /// The model score of reading `letters` as exactly the word `units`.
    fn forced(&self, letters: &[u8], units: &[u8]) -> Option<f32> {
        let one = Fst::from_iter_set([units]).ok()?;
        decode::search(&self.model, letters, Some(&one), None, 64)
            .first()
            .map(|f| f.lm)
    }

    /// Records that the user chose `word` for `input`, so it ranks higher from
    /// now on, and first for the same letters. Returns false, and records
    /// nothing, if `word` is not a Kannada word.
    pub fn learn(&mut self, input: &str, word: &str) -> bool {
        let known = units::units(word).is_some_and(|u| self.lexicon_class(&u).is_some());
        roman::letters(input).is_some_and(|l| !l.is_empty()) && self.user.learn(input, word, known)
    }

    /// Forgets every pick of `word`.
    pub fn forget(&mut self, word: &str) {
        self.user.forget(word);
    }

    /// The user's picks as text, for the caller to store.
    pub fn export_user(&self) -> String {
        self.user.export()
    }

    /// Restores picks from [`Engine::export_user`]. Returns false, and keeps the
    /// current picks, if `text` is not such an export.
    pub fn import_user(&mut self, text: &str) -> bool {
        match User::import(text) {
            Some(u) => {
                self.user = u;
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(word: &str, lm: f32, known: bool, rule: bool, extra: usize) -> Candidate {
        Candidate {
            word: word.into(),
            lm,
            freq: 0.0,
            known,
            lift: 0.0,
            rule,
            precise: false,
            user: 0.0,
            picked: None,
            extra,
        }
    }

    fn order(mut c: Vec<Candidate>, letters: usize) -> Vec<String> {
        let w = Weights {
            known: 10.0,
            ..Weights::default()
        };
        rank(&mut c, &w, letters);
        c.into_iter().map(|c| c.word).collect()
    }

    #[test]
    fn short_input_leads_with_the_strict_reading_then_the_best_word() {
        let c = vec![
            cand("done+", 0.0, true, false, 2),
            cand("best", -1.0, true, false, 0),
            cand("strict", -5.0, false, true, 0),
        ];
        assert_eq!(order(c.clone(), 2), ["strict", "best", "done+"]);
        assert_eq!(order(c, 3)[0], "best");
    }

    #[test]
    fn a_likely_prefix_leads_with_the_best_fit_without_the_lexicon_bonus() {
        let c = vec![
            cand("word+", 5.0, true, false, 3),
            cand("known", -12.0, true, false, 0),
            cand("letters", -8.0, false, true, 0),
        ];
        assert_eq!(order(c, 5)[0], "letters");
        // With no completion ahead, the score decides.
        let c = vec![
            cand("known", -12.0, true, false, 0),
            cand("letters", -8.0, false, true, 0),
        ];
        assert_eq!(order(c, 5)[0], "known");
    }

    #[test]
    fn picks_stay_first_newest_first_and_the_strict_reading_follows_them() {
        let mut c: Vec<Candidate> = (0..6)
            .map(|i| Candidate {
                picked: Some(i),
                ..cand(&format!("p{i}"), -(i as f32), true, false, 0)
            })
            .collect();
        c.push(cand("best", 10.0, true, false, 0));
        c.push(cand("strict", -50.0, false, true, 0));
        let got = order(c, 2);
        assert_eq!(got[..6], ["p5", "p4", "p3", "p2", "p1", "p0"]);
        assert_eq!(got[6], "strict");
    }

    #[test]
    fn equal_fits_keep_the_score_order() {
        let c = vec![
            cand("word+", 5.0, true, false, 3),
            cand("a", -8.0, false, false, 0),
            cand("b", -8.0, false, false, 0),
        ];
        assert_eq!(order(c, 5)[0], "a");
    }

    #[test]
    fn the_strict_reading_moves_up_to_the_fifth_place() {
        let mut c: Vec<Candidate> = (0..8)
            .map(|i| cand(&format!("w{i}"), -(i as f32), true, false, 0))
            .collect();
        c.push(cand("strict", -50.0, false, true, 0));
        let got = order(c, 5);
        assert_eq!(got[4], "strict");
        assert_eq!(got[..4], ["w0", "w1", "w2", "w3"]);
    }
}
