//! Building, evaluating and tuning a model. Needs the `train` feature.

mod align;
pub mod data;
pub mod eval;
mod kn;

use crate::lm::Quant;
use crate::model::{freq_class, Graphone, Model, Pairs, Weights};
use crate::units::Unit;
use align::{Aligner, Pair};
use fst::MapBuilder;
use std::collections::{BTreeMap, HashMap};

/// What to build a model from.
pub struct Inputs {
    /// Latin letters and units of each training pair.
    pub pairs: Vec<(Vec<u8>, Vec<Unit>)>,
    /// Lexicon words as units, with their corpus counts.
    pub lexicon: BTreeMap<Vec<Unit>, u64>,
    /// Sentences of Kannada words, for the word pairs.
    pub sentences: Vec<Vec<String>>,
}

/// Build settings.
pub struct Settings {
    pub order: usize,
    pub min_count: Vec<u32>,
    pub em_iterations: usize,
    /// Pairs whose best alignment uses a graphone seen fewer times than this are dropped as noise.
    pub min_graphone: u32,
    /// Word pairs seen fewer times than this are not kept.
    pub min_pair: u32,
    /// At most this many word pairs are kept, the commonest first.
    pub max_pairs: usize,
    pub threads: usize,
}

/// Aligns the pairs and returns each pair's graphone sequence.
fn align(pairs: &[Pair], settings: &Settings) -> Vec<Vec<Graphone>> {
    let aligner = Aligner::train(pairs, settings.em_iterations, settings.threads);
    pairs
        .iter()
        .filter_map(|(r, u)| aligner.best(r, u).map(|(g, _)| g))
        .collect()
}

/// Builds a model, logging progress to standard error.
pub fn build(inputs: &Inputs, settings: &Settings) -> Model {
    let t = std::time::Instant::now();
    let aligned = align(&inputs.pairs, settings);
    eprintln!(
        "aligned {} of {} pairs in {:.0?}",
        aligned.len(),
        inputs.pairs.len(),
        t.elapsed()
    );
    let mut seen: HashMap<Graphone, u32> = HashMap::new();
    aligned
        .iter()
        .flatten()
        .for_each(|g| *seen.entry(*g).or_default() += 1);
    let mut graphones: Vec<Graphone> = seen
        .iter()
        .filter(|&(_, &c)| c >= settings.min_graphone)
        .map(|(&g, _)| g)
        .collect();
    graphones.sort();
    graphones.insert(0, Graphone { chunk: 0, unit: 0 });
    let id: HashMap<Graphone, u16> = graphones
        .iter()
        .enumerate()
        .skip(1)
        .map(|(i, &g)| (g, i as u16))
        .collect();
    let seqs: Vec<Vec<u16>> = aligned
        .iter()
        .filter_map(|s| {
            s.iter()
                .map(|g| id.get(g).copied())
                .collect::<Option<Vec<u16>>>()
        })
        .collect();
    eprintln!(
        "{} graphones, {} sequences after dropping rare graphones",
        graphones.len() - 1,
        seqs.len()
    );
    let lm = kn::estimate(
        &seqs,
        graphones.len(),
        &kn::Settings {
            order: settings.order,
            min_count: settings.min_count.clone(),
        },
    );
    let sizes: Vec<usize> = lm.levels.iter().map(|l| l.prob.len()).collect();
    eprintln!("n-grams per order {sizes:?} after {:.0?}", t.elapsed());
    let mut builder = MapBuilder::memory();
    for (word, &count) in &inputs.lexicon {
        builder
            .insert(word, freq_class(count))
            .expect("sorted lexicon");
    }
    let lexicon = fst::raw::Fst::new(builder.into_inner().expect("lexicon")).expect("lexicon");
    let pairs = word_pairs(&inputs.sentences, settings);
    eprintln!(
        "lexicon {} words, {} bytes; {} word pairs",
        inputs.lexicon.len(),
        lexicon.as_bytes().len(),
        pairs.keys.len()
    );
    Model::new(graphones, lm, lexicon, pairs, Weights::default())
}

/// Word pairs that occur together more than chance, with their lift.
fn word_pairs(sentences: &[Vec<String>], settings: &Settings) -> Pairs {
    let mut uni: HashMap<&str, u64> = HashMap::new();
    let mut bi: HashMap<(&str, &str), u32> = HashMap::new();
    let mut total = 0u64;
    for s in sentences {
        for (i, w) in s.iter().enumerate() {
            *uni.entry(w).or_default() += 1;
            total += 1;
            if i > 0 {
                *bi.entry((&s[i - 1], w)).or_default() += 1;
            }
        }
    }
    let mut kept: Vec<((&str, &str), u32, f32)> = bi
        .into_iter()
        .filter(|&(_, c)| c >= settings.min_pair)
        .map(|((a, b), c)| {
            let lift = (c as f64 * total as f64 / (uni[a] as f64 * uni[b] as f64)).ln() as f32;
            ((a, b), c, lift)
        })
        .filter(|&(_, _, lift)| lift > 0.5)
        .collect();
    kept.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    kept.truncate(settings.max_pairs);
    let max = kept.iter().map(|k| k.2).fold(0f32, f32::max);
    let q = Quant::new(0.0, max.max(1.0));
    let mut entries: BTreeMap<u32, u8> = BTreeMap::new();
    for ((a, b), _, lift) in kept {
        entries.entry(Pairs::key(a, b)).or_insert(q.encode(lift));
    }
    Pairs {
        keys: entries.keys().copied().collect(),
        values: entries.values().copied().collect(),
        q: (!entries.is_empty()).then_some(q),
    }
}
