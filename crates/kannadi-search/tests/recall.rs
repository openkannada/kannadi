//! Recall and precision on the Wikipedia test set in eval/data, as the README reports
//! them. eval/README.md says how the set was made; the development set beside it is for
//! looking at errors.

use std::collections::{HashMap, HashSet};
use std::fs;

use kannadi_search::Analyzer;

fn read(name: &str) -> String {
    fs::read_to_string(format!("{}/eval/data/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
}

fn docs(name: &str) -> Vec<(String, String)> {
    read(name)
        .lines()
        .skip(1)
        .map(|l| {
            let (id, text) = l.split_once('\t').unwrap();
            (id.to_string(), text.to_string())
        })
        .collect()
}

fn queries() -> Vec<(String, Vec<String>)> {
    read("wiki-queries.tsv")
        .lines()
        .skip(1)
        .map(|l| {
            let (q, ids) = l.split_once('\t').unwrap();
            (q.to_string(), ids.split(',').map(String::from).collect())
        })
        .collect()
}

/// Recall and precision of an AND search over the analyzer's tokens.
fn score(analyzer: Analyzer, docs: &[(String, String)]) -> (f64, f64) {
    let mut index: HashMap<String, HashSet<&str>> = HashMap::new();
    for (id, text) in docs {
        for key in analyzer.keys(text) {
            index.entry(key).or_default().insert(id);
        }
    }
    let (mut hits, mut found, mut relevant) = (0, 0, 0);
    for (query, gold) in queries() {
        let mut result: Option<HashSet<&str>> = None;
        for key in analyzer.keys(&query) {
            let ids = index.get(&key).cloned().unwrap_or_default();
            result = Some(match result {
                None => ids,
                Some(r) => r.intersection(&ids).copied().collect(),
            });
        }
        let result = result.unwrap_or_default();
        hits += gold
            .iter()
            .filter(|id| result.contains(id.as_str()))
            .count();
        found += result.len();
        relevant += gold.len();
    }
    (hits as f64 / relevant as f64, hits as f64 / found as f64)
}

#[test]
fn stemming_finds_inflected_forms() {
    for set in ["wiki-docs.tsv", "wiki-variant-docs.tsv"] {
        let (recall, precision) = score(Analyzer::new(), &docs(set));
        eprintln!("{set}: recall {recall:.4}, precision {precision:.4}");
        assert!(recall >= 0.975, "{set}: recall {recall}");
        assert!(precision >= 0.98, "{set}: precision {precision}");
    }
}

#[test]
fn spelling_keys_make_variants_count_as_the_clean_text() {
    let analyzer = Analyzer::new().stemming(false);
    let clean = score(analyzer, &docs("wiki-docs.tsv"));
    let varied = score(analyzer, &docs("wiki-variant-docs.tsv"));
    assert_eq!(clean, varied);
    assert!(clean.1 >= 0.999, "precision {}", clean.1);
}

#[test]
fn every_key_is_its_own_key() {
    let analyzer = Analyzer::new();
    for (_, text) in docs("wiki-variant-docs.tsv") {
        for key in analyzer.keys(&text) {
            assert_eq!(analyzer.keys(&key), std::slice::from_ref(&key), "in {text}");
        }
    }
}
