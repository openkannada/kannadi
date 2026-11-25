# kannadi-search

Search analysis for Kannada text. `kannadi-search` turns text into search tokens so that a word is found in all its spellings and in its case and plural forms: a search for ಮರ ("tree") finds ಮರಗಳಲ್ಲಿ ("in the trees"), ಕನ್ನಡ finds ಕನ್‌ನಡ typed with a joiner, and a search in Unicode finds text from old government PDFs typed in Nudi or Baraha fonts.

```rust
use kannadi_search::Analyzer;

let analyzer = Analyzer::new();
assert_eq!(analyzer.keys("ಮರಗಳಲ್ಲಿ ಹಕ್ಕಿಗಳಿವೆ"), ["ಮರ", "ಹಕ್ಕಿ"]);
assert_eq!(analyzer.keys("ಕನ್\u{200C}ನಡದ"), analyzer.keys("ಕನ್ನಡ"));
assert_eq!(analyzer.keys("PÀ£ÀßqÀ"), ["ಕನ್ನಡ"]);
```

It plugs into [Tantivy](#tantivy) as a `TextAnalyzer` and into [Postgres](examples/postgres/README.md) full-text search. For Meilisearch, a contribution to its tokenizer is being prepared ([below](#meilisearch)).

## Results

Recall and precision when searching for a noun in 1 spelling, measured on 3 document sets:

- **Wikipedia**: 3,000 sentences from held-out Kannada Wikipedia articles, as written.
- **Wikipedia, variant spellings**: the same sentences with spelling variants put in at random: joiners after a virama, ೦ for ಂ, a nasal with virama for ಂ (ಸಮ್ಬನ್ಧ), Old Kannada ೞ and ಱ, a nukta on ಜ and ಫ, vowel signs typed in 2 parts, and Kannada digits.
- **Government PDFs**: the 5,098 lines of the Open Kannada legacy-encodings benchmark, from 275 public Karnataka government PDFs, as text extraction gives them: legacy font codes, such as PÀ£ÀßqÀ for ಕನ್ನಡ. The same lines in Unicode are the last column.

Each cell is recall / precision, in percent. Queries are 1,000 nouns per set; a document is relevant when it holds the noun in any form ([how relevance is decided](eval/README.md#relevance)). n/a: the analyzer found nothing.

| Analyzer | Wikipedia | Wikipedia, variant spellings | Government PDFs, legacy text | Government lines in Unicode |
|---|---:|---:|---:|---:|
| Whitespace split | 41.1 / 100.0 | 28.0 / 100.0 | 0.0 / n/a | 50.4 / 100.0 |
| Tantivy default (`SimpleTokenizer`, `LowerCaser`) | 50.0 / 58.5 | 42.0 / 53.4 | 0.0 / n/a | 63.1 / 75.7 |
| ICU (ICU4X word segmenter, NFKC_Casefold) | 46.8 / 100.0 | 41.0 / 100.0 | 0.0 / n/a | 61.2 / 100.0 |
| Meilisearch default (charabia 0.10.0) | 47.8 / 99.1 | 39.6 / 98.3 | 0.0 / n/a | 62.0 / 99.9 |
| kannadi-search, spelling only | 47.0 / 100.0 | 47.0 / 100.0 | 61.0 / 100.0 | 62.0 / 100.0 |
| **kannadi-search** | **98.5 / 98.6** | **98.5 / 98.6** | **95.9 / 97.4** | **97.4 / 97.3** |

What the numbers show:

- **Endings are most of the gap.** A Kannada noun takes case, plural and clitic endings, and Kannada writes "is" and "become" (ಇದೆ, ಆಗಿ) joined to the noun: ಮರ appears as ಮರದ, ಮರಗಳಲ್ಲಿ, ಮರವಾಗಿದೆ. Without stemming, a search finds about half of the documents that use the word.
- **Spelling variants cost the other analyzers 6 to 13 points of recall.** ICU removes joiners through NFKC_Casefold but keeps the nukta, ೦ and Old Kannada letters apart. charabia keeps joiners in tokens and splits words at a ZWNJ (ಕನ್‌ನಡ becomes ಕನ್ and ನಡ).
- **Tantivy's default tokenizer splits words at the virama.** `SimpleTokenizer` keeps runs of alphanumeric characters, and the virama is not alphanumeric, so ಕನ್ನಡ becomes ಕನ and ನಡ. Its recall looks higher because such fragments match many words; its precision is 53% to 76%.
- **Legacy text is invisible to every other analyzer.** On text extracted from legacy-font PDFs they find nothing. kannadi-search converts legacy words with kannadi's converter and loses 1.5 points against the same lines in Unicode.
- **Precision stays near 98%.** Many of the remaining false matches are not wrong: the reference lexicon has no entry for some forms (ಚರ್ಚ್ and ಚರ್ಚು are the same "church"), and it files adverbs such as ಪ್ರಮುಖವಾಗಿ under adjectives, which are no query. Others are real conflations, such as ಹಿಂದು ("Hindu") with ಹಿಂದಕ್ಕೆ ("backwards").

Limits of the measurement:

- Relevance comes from kannada-morph's analyzer with its full lexicon (Alar and Kannada Wikipedia), and the stemmer uses the same analyzer's endings without the lexicon. The baselines do no stemming at all, so their order does not depend on this, but the stemmer's recall is measured against the paradigms it was built from: endings kannada-morph lacks are missing from both.
- The variant set holds the variants kannadi-search was built to fold, so its equal score on clean and variant text shows that the folding works, and says nothing of variants it does not know.
- The stemmer's rules and data were refined while reading errors on a separate development set of 3,000 sentences; the Wikipedia columns come from a test set that was only scored. The 2 sets share no sentence: repeated sentences were dropped before sampling. The query filters described in eval/README.md help a stemmer slightly. I also read errors on the government set while building the first version, so its numbers are less strict.
- Queries are nouns. Verbs are out of scope: ಮಾಡಿದರು does not find ಮಾಡು.
- The government lines' Unicode text is the benchmark's expected text, mostly confirmed by OCR and not yet checked by a reader. That set lives in a private repository, so it is not in this repository and CI does not run it; the Wikipedia sets are in `eval/data` and CI checks their numbers.

Reproduce the table with the steps in [eval/README.md](eval/README.md).

## How it works

[`Analyzer::analyze`](src/lib.rs) runs 4 steps.

1. **Legacy text** ([`legacy`](src/legacy.rs)). A line is read as legacy font codes when it has at least 3 Windows-1252 characters above ASCII (quotes and dashes aside), they are at least 40% of the line or 2 of its words have 2 each, and the words that have them convert cleanly in the best layout (Nudi and Baraha, Akruti, Shree-Lipi, Nudi B-Akshara or Kan-Eng). Accented European words, such as Ñúñez, do not convert cleanly, and French guillemets or N° alone are too few. A line that is mostly legacy words converts whole; in a line that mixes them with English, only the legacy words convert. Words convert 1 at a time, so each token keeps the place of its legacy word.
2. **Words**. Text splits at Unicode word boundaries ([UAX #29](https://www.unicode.org/reports/tr29/)), which never fall inside an akshara: vowel signs, the virama, the nukta and joiners stay with their letter. A property test checks that every token starts and ends at an akshara boundary. Kannada written straight after a number or a word in another script is its case ending and is dropped: ೨೦೨೬ರಲ್ಲಿ ("in 2026") gives 2026, and Facebookನಲ್ಲಿ gives facebook. A zero width space inside a Kannada word does not split it.
3. **Spelling** ([`fold`](src/fold.rs)). Each word becomes its search key: the search form of kannadi's [normalization profile](../../NORMALIZATION.md#search-form) (joiners, the nukta, Old Kannada letters, ೦ typed for ಂ, vowel signs in 2 parts, and the other canonical rules), then a nasal with virama before a consonant of its own group becomes ಂ (ಸಮ್ಬನ್ಧ, ಸಂಬಂಧ), Kannada digits become ASCII digits, and other letters are lowercased.
4. **Endings** ([`stem`](src/stem.rs)). Case, plural and clitic endings, and ಆಗು and ಇರು written joined to a noun, are removed: ಮರಗಳಲ್ಲಿ, ಮರವನ್ನು and ಮರವಾಗಿದೆ all give ಮರ.

The stemmer knows no dictionary. It runs kannada-morph's noun paradigms backwards and is careful in 4 ways:

- A stem keeps at least 2 aksharas: in shorter words an ending is most often part of the word.
- 1,278 words that end like an inflected noun but are stems of their own stay whole, such as ಮೆರವಣಿಗೆ ("procession"), which is not ಮೆರವಣಿ with a dative ending. A word is on this list when Kannada Wikipedia has at least 2 forms built on it whose stemming passes through it, those forms are together at least 1 tenth as frequent as the word, and the shorter stem is rarer as a word.
- When endings give more than 1 stem, the stem Kannada Wikipedia uses most as a word wins: ವ್ಯಕ್ತಿಗೆ ("to the person") reads as ವ್ಯಕ್ತಿ + ಗೆ, as ಕಾಡಿಗೆ reads as ಕಾಡು + ಇಗೆ. 3,781 stems carry the counts for this.
- Otherwise the longest ending wins, and when it leaves 2 different stems, the word stays whole.

Endings come off 1 at a time, up to 4 (ಮನೆಯನ್ನಾಗಿ, then ಮನೆಯನ್ನು, then ಮನೆ). A key is a search key and need not be a dictionary headword: a final ು after a consonant becomes a virama, so a loanword and its ು form fold together (ಬ್ಯಾಂಕ್, ಬ್ಯಾಂಕು) and ಕಾಡು's key is ಕಾಡ್. Analyzing a key again gives the same key, which a test checks on every word of the test set.

Use the same analyzer for documents and queries. `Analyzer::new().stemming(false)` gives spelling keys alone, for fields where endings matter; `.legacy(false)` skips legacy detection.

## Data

| File | What | From |
|---|---|---|
| `data/suffixes.tsv` | 1,521 noun endings: stem final, characters to strip, characters to add | kannada-morph's paradigms ([kannada-spell](https://github.com/openkannada/kannada-spell) at 9366c23), by `eval` |
| `data/protected.txt` | 1,278 words the stemmer keeps whole | Kannada Wikipedia word counts of the training articles, by `eval` |
| `data/preferred.tsv` | 3,781 stems with their Wikipedia counts | the same counts, by `eval` |
| `eval/data/` | the Wikipedia development and test sets: sentences, variant sentences, queries and relevant documents | held-out Kannada Wikipedia articles |

The counts come from the Kannada Wikipedia dump of 1 November 2026, split by article as kannada-spell splits it: every 10th article is held out, and the evaluation sentences come only from held-out articles. The development and test sets are 2 disjoint samples of them.

## Tantivy

With the `tantivy` feature, [`tantivy::analyzer()`](src/tantivy.rs) is the whole pipeline as a `TextAnalyzer`: `KannadaTokenizer` (legacy text, words and spelling keys) and the `KannadaStemmer` filter. Register it and name it in the field's indexing options:

```rust
use tantivy::schema::{IndexRecordOption, Schema, TextFieldIndexing, TextOptions};
use tantivy::Index;

let indexing = TextFieldIndexing::default()
    .set_tokenizer("kannada")
    .set_index_option(IndexRecordOption::WithFreqsAndPositions);
let mut schema = Schema::builder();
let body = schema.add_text_field("body", TextOptions::default().set_indexing_options(indexing));
let index = Index::create_in_ram(schema.build());
index.tokenizers().register("kannada", kannadi_search::tantivy::analyzer());
```

Token offsets point at the original word, legacy text included, so highlighting works. `KannadaTokenizer::default().legacy(false)` skips legacy detection. A test indexes Kannada, variant and legacy documents and checks what each query finds. Tantivy's `RemoveLongFilter` can follow the stemmer to drop very long tokens.

## Meilisearch

Meilisearch tokenizes with [charabia](https://github.com/meilisearch/charabia). A Kannada normalizer and segmenter for charabia, self-contained and covering the spelling step, are being prepared as a contribution there. Stemming is not part of it, as charabia stems no language.

## Postgres

[examples/postgres](examples/postgres/README.md) has 2 setups, both run on Postgres 19: an `unaccent` rules file that folds spelling variants inside Postgres full-text search, and a `tsvector` column of kannadi-search keys written by the application, which adds endings and legacy text.

## Speed

1 pass over all 99,579 distinct held-out Wikipedia sentences (30 MB), each analyzer on its own thread after a warm-up, on an Apple M1 Pro that was also running other work: kannadi-search 17.5 MB/s without stemming and 10.8 MB/s with it; ICU4X 20.4 MB/s, charabia 19.0 MB/s, Tantivy's default 12.0 MB/s. Most of kannadi-search's time goes to normalization and stemming; each thread keeps the keys of up to 65,536 words and empties the store when it is full, which helps on text that repeats words. Lines in legacy fonts run at about 0.8 MB/s, because choosing the layout converts them in every layout.
