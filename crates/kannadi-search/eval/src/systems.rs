//! The analyzers compared: each turns text into the tokens a search index stores.

use charabia::Tokenize;
use icu_casemap::CaseMapperBorrowed;
use icu_normalizer::ComposingNormalizerBorrowed;
use icu_segmenter::options::WordBreakInvariantOptions;
use icu_segmenter::WordSegmenter;
use kannadi_search::Analyzer;
use tantivy::tokenizer::{LowerCaser, SimpleTokenizer, TextAnalyzer};

/// Text to the tokens an index stores.
pub type Tokens = Box<dyn Fn(&str) -> Vec<String>>;

pub struct System {
    pub name: &'static str,
    pub tokens: Tokens,
}

pub fn all() -> Vec<System> {
    vec![
        System {
            name: "Whitespace",
            tokens: Box::new(|t| t.split_whitespace().map(String::from).collect()),
        },
        System {
            name: "Tantivy default (SimpleTokenizer, LowerCaser)",
            tokens: Box::new(tantivy_default),
        },
        System {
            name: "ICU (ICU4X word segmenter, NFKC_Casefold)",
            tokens: Box::new(icu),
        },
        System {
            name: "Meilisearch default (charabia 0.10.0)",
            tokens: Box::new(|t| {
                t.tokenize()
                    .filter(|tok| tok.is_word())
                    .map(|tok| tok.lemma().to_string())
                    .collect()
            }),
        },
        System {
            name: "kannadi-search, spelling only",
            tokens: Box::new(|t| Analyzer::new().stemming(false).keys(t)),
        },
        System {
            name: "kannadi-search, spelling and stemming",
            tokens: Box::new(|t| Analyzer::new().keys(t)),
        },
        System {
            name: "kannadi-search, no legacy conversion",
            tokens: Box::new(|t| Analyzer::new().legacy(false).keys(t)),
        },
    ]
}

fn tantivy_default(text: &str) -> Vec<String> {
    let mut analyzer = TextAnalyzer::builder(SimpleTokenizer::default())
        .filter(LowerCaser)
        .build();
    let mut stream = analyzer.token_stream(text);
    let mut out = Vec::new();
    while stream.advance() {
        out.push(stream.token().text.clone());
    }
    out
}

/// What Elasticsearch's icu_tokenizer and icu_normalizer (nfkc_cf) do: UAX #29 words,
/// then NFKC, case folding and removal of default-ignorable characters such as ZWNJ.
fn icu(text: &str) -> Vec<String> {
    let segmenter = WordSegmenter::new_auto(WordBreakInvariantOptions::default());
    let nfkc = ComposingNormalizerBorrowed::new_nfkc();
    let case = CaseMapperBorrowed::new();
    let breaks: Vec<usize> = segmenter.segment_str(text).collect();
    breaks
        .windows(2)
        .map(|w| &text[w[0]..w[1]])
        .filter(|w| w.chars().any(char::is_alphanumeric))
        .map(|w| {
            let cleaned: String = case
                .fold_string(&nfkc.normalize(w))
                .chars()
                .filter(|c| {
                    !matches!(
                        c,
                        '\u{00AD}' | '\u{200B}'..='\u{200F}' | '\u{2060}' | '\u{FEFF}'
                    )
                })
                .collect();
            nfkc.normalize(&cleaned).into_owned()
        })
        .collect()
}
