//! `gold`: which documents each query word should find, by kannada-morph's analyzer
//! with its full lexicon.
//!
//! A document is relevant to a noun lemma when 1 of its words is that lemma or has an
//! analysis with it as lemma, in any form, fused predicates included (ಮರವಾಗಿದೆ is
//! relevant to ಮರ). Words are read from the clean text in the search form of kannadi's
//! normalization profile, so a spelling variant already in the text counts as its word.
//! Queries are noun lemmas of 2 or more aksharas found in 2 or more documents, that
//! Kannada Wikipedia's training articles have as a word at least 10 times: the lexicon
//! also holds stems no one types alone, such as ಮತ್ (of ಮತ್ತು). A lemma
//! that is itself a form of another lemma (the lexicon lists ಮರವು beside ಮರ) is not a
//! query: the 2 entries split 1 word's documents between them.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use kannada_morph::{Analyzer, Lexicon};

use crate::variants::Rng;

/// How often a query word must appear as a word in the Wikipedia training articles.
pub const MIN_QUERY_COUNT: u64 = 10;

pub struct Gold {
    /// Query word, and the ids of the documents relevant to it.
    pub queries: Vec<(String, Vec<String>)>,
}

pub fn build(
    lexicon: &str,
    counts: &HashMap<String, u64>,
    docs: &[(String, String)],
    max_queries: usize,
    seed: u64,
) -> Gold {
    let analyzer = Analyzer::new(Lexicon::parse(lexicon).expect("lexicon"));
    let mut cache: HashMap<String, Vec<String>> = HashMap::new();
    let mut relevant: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for (id, text) in docs {
        let text = kannadi::normalize::normalize_as(text, kannadi::normalize::Form::Search);
        for word in kannada_morph::script::words(&text) {
            let lemmas = cache.entry(word.clone()).or_insert_with(|| {
                let mut lemmas: Vec<String> = analyzer
                    .analyze(&word)
                    .into_iter()
                    .filter(|a| a.pos == "n")
                    .map(|a| a.lemma)
                    .collect();
                lemmas.sort();
                lemmas.dedup();
                lemmas
            });
            for lemma in lemmas.iter() {
                relevant.entry(lemma.clone()).or_default().insert(id.clone());
            }
        }
    }
    let mut candidates: Vec<(String, Vec<String>)> = relevant
        .into_iter()
        .filter(|(lemma, ids)| ids.len() >= 2 && kannadi::count_aksharas(lemma) >= 2)
        .filter(|(lemma, _)| analyzer.analyze(lemma).iter().all(|a| a.lemma == *lemma))
        .filter(|(lemma, _)| counts.get(lemma).is_some_and(|&n| n >= MIN_QUERY_COUNT))
        .map(|(lemma, ids)| (lemma, ids.into_iter().collect()))
        .collect();
    let mut rng = Rng::new(seed);
    // A fixed random sample, so frequent and rare words both appear.
    for i in (1..candidates.len()).rev() {
        let j = (rng.next() % (i as u64 + 1)) as usize;
        candidates.swap(i, j);
    }
    candidates.truncate(max_queries);
    candidates.sort();
    Gold { queries: candidates }
}

pub fn write(gold: &Gold) -> String {
    let mut out = String::from("query\trelevant\n");
    for (q, ids) in &gold.queries {
        out.push_str(&format!("{q}\t{}\n", ids.join(",")));
    }
    out
}

pub fn read(text: &str) -> Vec<(String, Vec<String>)> {
    text.lines()
        .skip(1)
        .filter_map(|l| {
            let (q, ids) = l.split_once('\t')?;
            Some((q.to_string(), ids.split(',').map(String::from).collect()))
        })
        .collect()
}
