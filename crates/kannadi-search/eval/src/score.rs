//! `score`: recall and precision of each system on 1 document set.
//!
//! A system finds a document for a query when the document's tokens include every token
//! of the query, as an AND search does. Recall and precision are summed over all queries
//! (micro averages).

use std::collections::{HashMap, HashSet};

use crate::systems::System;

/// Descriptions with how often each occurred, most frequent first.
pub type Tally = Vec<(String, usize)>;

pub struct Score {
    pub recall: f64,
    pub precision: f64,
    pub found: usize,
    pub relevant: usize,
}

pub fn score(
    system: &System,
    docs: &[(String, String)],
    queries: &[(String, Vec<String>)],
) -> Score {
    let mut index: HashMap<String, HashSet<usize>> = HashMap::new();
    for (i, (_, text)) in docs.iter().enumerate() {
        for token in (system.tokens)(text) {
            index.entry(token).or_default().insert(i);
        }
    }
    let ids: HashMap<&str, usize> = docs
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (id.as_str(), i))
        .collect();
    let (mut hits, mut found, mut relevant) = (0, 0, 0);
    for (query, gold) in queries {
        let gold: HashSet<usize> = gold
            .iter()
            .filter_map(|id| ids.get(id.as_str()).copied())
            .collect();
        let tokens = (system.tokens)(query);
        let mut result: Option<HashSet<usize>> = None;
        for t in &tokens {
            let docs = index.get(t).cloned().unwrap_or_default();
            result = Some(match result {
                None => docs,
                Some(r) => r.intersection(&docs).copied().collect(),
            });
        }
        let result = result.unwrap_or_default();
        hits += result.intersection(&gold).count();
        found += result.len();
        relevant += gold.len();
    }
    Score {
        recall: hits as f64 / relevant.max(1) as f64,
        precision: hits as f64 / found.max(1) as f64,
        found,
        relevant,
    }
}

/// For the full analyzer: the document words behind false matches and the relevant
/// words it missed, each with how often, most frequent first.
pub fn errors(docs: &[(String, String)], queries: &[(String, Vec<String>)]) -> (Tally, Tally) {
    let analyzer = kannadi_search::Analyzer::new();
    let ids: HashMap<&str, usize> = docs
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (id.as_str(), i))
        .collect();
    let words: Vec<Vec<(String, String)>> = docs
        .iter()
        .map(|(_, t)| {
            kannada_morph::script::words(t)
                .map(|w| (analyzer.key(&w), w))
                .collect()
        })
        .collect();
    let (mut false_hits, mut misses): (HashMap<String, usize>, HashMap<String, usize>) =
        Default::default();
    for (query, gold) in queries {
        let key = analyzer.key(query);
        let gold: HashSet<usize> = gold
            .iter()
            .filter_map(|id| ids.get(id.as_str()).copied())
            .collect();
        for (i, doc) in words.iter().enumerate() {
            let matched: Vec<&String> = doc
                .iter()
                .filter(|(k, _)| *k == key)
                .map(|(_, w)| w)
                .collect();
            if !gold.contains(&i) {
                for w in matched {
                    *false_hits.entry(format!("{query} <- {w}")).or_default() += 1;
                }
            } else if matched.is_empty() {
                // a relevant document the analyzer missed: show its words with the same first 2 aksharas
                let prefix: String = kannadi::split(query).into_iter().take(2).collect();
                for (_, w) in doc.iter().filter(|(_, w)| w.starts_with(&prefix)) {
                    *misses
                        .entry(format!("{query} -> {w} [{}]", analyzer.key(w)))
                        .or_default() += 1;
                }
            }
        }
    }
    let sort = |m: HashMap<String, usize>| {
        let mut v: Vec<(String, usize)> = m.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        v
    };
    (sort(false_hits), sort(misses))
}
