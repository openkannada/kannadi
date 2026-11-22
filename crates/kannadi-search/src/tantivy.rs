//! Tantivy: a tokenizer and a stemming filter for Kannada fields.
//!
//! [`analyzer`] is the whole pipeline as a [`TextAnalyzer`]. Register it under a name
//! and give that name to the fields that hold Kannada text:
//!
//! ```
//! use tantivy::schema::{IndexRecordOption, Schema, TextFieldIndexing, TextOptions};
//! use tantivy::Index;
//!
//! let indexing = TextFieldIndexing::default()
//!     .set_tokenizer("kannada")
//!     .set_index_option(IndexRecordOption::WithFreqsAndPositions);
//! let mut schema = Schema::builder();
//! schema.add_text_field("body", TextOptions::default().set_indexing_options(indexing));
//! let index = Index::create_in_ram(schema.build());
//! index.tokenizers().register("kannada", kannadi_search::tantivy::analyzer());
//! ```
//!
//! [`KannadaTokenizer`] alone gives the spelling keys, for fields where endings matter;
//! [`KannadaStemmer`] can follow any tokenizer.

use tantivy::tokenizer::{TextAnalyzer, Token, TokenFilter, TokenStream, Tokenizer};

use crate::Analyzer;

/// The tokenizer and the stemmer: what [`Analyzer::new`] does.
pub fn analyzer() -> TextAnalyzer {
    TextAnalyzer::builder(KannadaTokenizer::default())
        .filter(KannadaStemmer)
        .build()
}

/// Splits text into words and gives each its spelling key ([`crate::fold`]),
/// converting legacy font lines first ([`crate::legacy`]).
#[derive(Clone, Debug)]
pub struct KannadaTokenizer {
    analyzer: Analyzer,
    tokens: Vec<Token>,
}

impl Default for KannadaTokenizer {
    fn default() -> Self {
        Self::new(true)
    }
}

impl KannadaTokenizer {
    /// A tokenizer that converts legacy font lines when `legacy` is true.
    pub fn new(legacy: bool) -> Self {
        Self {
            analyzer: Analyzer::new().stemming(false).legacy(legacy),
            tokens: Vec::new(),
        }
    }
}

impl Tokenizer for KannadaTokenizer {
    type TokenStream<'a> = KannadaTokenStream<'a>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> KannadaTokenStream<'a> {
        self.tokens.clear();
        self.tokens
            .extend(self.analyzer.analyze(text).into_iter().map(|t| Token {
                offset_from: t.offsets.start,
                offset_to: t.offsets.end,
                position: t.position,
                text: t.text,
                position_length: 1,
            }));
        KannadaTokenStream {
            tokens: &mut self.tokens,
            next: 0,
        }
    }
}

/// The tokens of 1 text, from [`KannadaTokenizer`].
pub struct KannadaTokenStream<'a> {
    tokens: &'a mut Vec<Token>,
    next: usize,
}

impl TokenStream for KannadaTokenStream<'_> {
    fn advance(&mut self) -> bool {
        self.next += 1;
        self.next <= self.tokens.len()
    }

    fn token(&self) -> &Token {
        &self.tokens[self.next - 1]
    }

    fn token_mut(&mut self) -> &mut Token {
        &mut self.tokens[self.next - 1]
    }
}

/// Removes case and plural endings from Kannada tokens ([`crate::stem`]). Give it
/// tokens in their spelling key, as [`KannadaTokenizer`] makes them; other tokens
/// pass unchanged.
#[derive(Clone, Copy, Debug, Default)]
pub struct KannadaStemmer;

impl TokenFilter for KannadaStemmer {
    type Tokenizer<T: Tokenizer> = KannadaStemmerFilter<T>;

    fn transform<T: Tokenizer>(self, tokenizer: T) -> KannadaStemmerFilter<T> {
        KannadaStemmerFilter(tokenizer)
    }
}

/// A tokenizer followed by [`KannadaStemmer`].
#[derive(Clone, Debug)]
pub struct KannadaStemmerFilter<T>(T);

impl<T: Tokenizer> Tokenizer for KannadaStemmerFilter<T> {
    type TokenStream<'a> = KannadaStemmerStream<T::TokenStream<'a>>;

    fn token_stream<'a>(&'a mut self, text: &'a str) -> Self::TokenStream<'a> {
        KannadaStemmerStream(self.0.token_stream(text))
    }
}

/// The tokens of 1 text, stemmed.
pub struct KannadaStemmerStream<S>(S);

impl<S: TokenStream> TokenStream for KannadaStemmerStream<S> {
    fn advance(&mut self) -> bool {
        if !self.0.advance() {
            return false;
        }
        let token = self.0.token_mut();
        token.text = crate::stem::stem(&token.text);
        true
    }

    fn token(&self) -> &Token {
        self.0.token()
    }

    fn token_mut(&mut self) -> &mut Token {
        self.0.token_mut()
    }
}

#[cfg(test)]
mod tests {
    use tantivy::collector::TopDocs;
    use tantivy::query::QueryParser;
    use tantivy::schema::{
        IndexRecordOption, Schema, TextFieldIndexing, TextOptions, Value, STORED,
    };
    use tantivy::{doc, Index, TantivyDocument};

    use super::*;

    fn tokens(analyzer: &mut TextAnalyzer, text: &str) -> Vec<(String, usize, usize, usize)> {
        let mut stream = analyzer.token_stream(text);
        let mut out = Vec::new();
        while let Some(t) = stream.next() {
            out.push((t.text.clone(), t.offset_from, t.offset_to, t.position));
        }
        out
    }

    #[test]
    fn analyzer_matches_the_library() {
        let text = "ಹಳೆಯ ಮರಗಳಲ್ಲಿ ಕನ್\u{200C}ನಡದ ೨೦೨೬ರಲ್ಲಿ\nPÀ£ÀßqÀ";
        let got = tokens(&mut analyzer(), text);
        let want: Vec<_> = Analyzer::new()
            .analyze(text)
            .into_iter()
            .map(|t| (t.text, t.offsets.start, t.offsets.end, t.position))
            .collect();
        assert_eq!(got, want);
        assert_eq!(got[1].0, "ಮರ");
        assert_eq!(&text[got[1].1..got[1].2], "ಮರಗಳಲ್ಲಿ");
    }

    #[test]
    fn the_tokenizer_alone_keeps_endings() {
        let mut plain = TextAnalyzer::from(KannadaTokenizer::default());
        assert_eq!(tokens(&mut plain, "ಮರಗಳಲ್ಲಿ")[0].0, "ಮರಗಳಲ್ಲಿ");
        let mut no_legacy = TextAnalyzer::from(KannadaTokenizer::new(false));
        assert!(tokens(&mut no_legacy, "PÀ£ÀßqÀ")
            .iter()
            .all(|t| t.0 != "ಕನ್ನಡ"));
    }

    #[test]
    fn a_kannada_index_finds_every_form_and_spelling() {
        let indexing = TextFieldIndexing::default()
            .set_tokenizer("kannada")
            .set_index_option(IndexRecordOption::WithFreqsAndPositions);
        let mut builder = Schema::builder();
        let body = builder.add_text_field(
            "body",
            TextOptions::default()
                .set_indexing_options(indexing)
                .set_stored(),
        );
        let id = builder.add_u64_field("id", STORED);
        let index = Index::create_in_ram(builder.build());
        index.tokenizers().register("kannada", analyzer());
        let mut writer = index.writer(15_000_000).unwrap();
        for (n, text) in [
            (1u64, "ಊರಿನ ಮರಗಳಲ್ಲಿ ಹಕ್ಕಿಗಳಿವೆ"),
            (2, "ಮರವನ್ನು ಕಡಿಯಬೇಡಿ"),
            (3, "ಕನ್\u{200C}ನಡ ಭಾಷೆ"),
            (4, "PÀ£ÀßqÀ ¨sÁµÉAiÀÄ ¥ÀÄ¸ÀÛPÀ"),
            (5, "ಮನೆಯ ಮುಂದೆ ಹೂವು"),
        ] {
            writer.add_document(doc!(id => n, body => text)).unwrap();
        }
        writer.commit().unwrap();
        let searcher = index.reader().unwrap().searcher();
        let parser = QueryParser::for_index(&index, vec![body]);
        let ids = |q: &str| {
            let query = parser.parse_query(q).unwrap();
            let mut ids: Vec<u64> = searcher
                .search(&query, &TopDocs::with_limit(10).order_by_score())
                .unwrap()
                .into_iter()
                .map(|(_, a)| {
                    let d: TantivyDocument = searcher.doc(a).unwrap();
                    d.get_first(id).and_then(|v| v.as_u64()).unwrap()
                })
                .collect();
            ids.sort();
            ids
        };
        assert_eq!(ids("ಮರ"), [1, 2]);
        assert_eq!(ids("ಕನ್ನಡ"), [3, 4]);
        assert_eq!(ids("ಭಾಷೆ"), [3, 4]);
        assert_eq!(ids("ಮನೆ"), [5]);
    }
}
