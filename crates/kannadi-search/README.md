# kannadi-search

Search analysis for Kannada text. `kannadi-search` turns text into search tokens so that a word is found in all its spellings and in its case and plural forms: a search for ಮರ ("tree") finds ಮರಗಳಲ್ಲಿ ("in the trees"), ಕನ್ನಡ finds ಕನ್‌ನಡ typed with a joiner, and a search in Unicode finds text from old government PDFs typed in Nudi or Baraha fonts.

```rust
use kannadi_search::Analyzer;

let analyzer = Analyzer::new();
assert_eq!(analyzer.keys("ಮರಗಳಲ್ಲಿ ಹಕ್ಕಿಗಳಿವೆ"), ["ಮರ", "ಹಕ್ಕಿ"]);
assert_eq!(analyzer.keys("ಕನ್\u{200C}ನಡದ"), analyzer.keys("ಕನ್ನಡ"));
assert_eq!(analyzer.keys("PÀ£ÀßqÀ"), ["ಕನ್ನಡ"]);
```

It plugs into [Tantivy](#tantivy) as a `TextAnalyzer`, into [Postgres](examples/postgres/README.md) full-text search, and, through a contribution being prepared for Meilisearch's tokenizer, [charabia](#meilisearch).

## Results

Recall and precision when searching for a noun in 1 spelling, measured on 3 document sets:

- **Wikipedia**: 3,000 sentences from held-out Kannada Wikipedia articles, as written.
- **Wikipedia, variant spellings**: the same sentences with spelling variants put in at random: joiners after a virama, ೦ for ಂ, a nasal with virama for ಂ (ಸಮ್ಬನ್ಧ), Old Kannada ೞ and ಱ, a nukta on ಜ and ಫ, vowel signs typed in 2 parts, and Kannada digits.
- **Government PDFs**: the 5,098 lines of the Open Kannada legacy-encodings benchmark, from 275 public Karnataka government PDFs, as text extraction gives them: legacy font codes, such as PÀ£ÀßqÀ for ಕನ್ನಡ. The same lines in Unicode are the last column.

Each cell is recall / precision. Queries are 1,000 nouns per set; a document is relevant when it holds the noun in any form ([how relevance is decided](eval/README.md#relevance)).

| Analyzer | Wikipedia | Wikipedia, variant spellings | Government PDFs, legacy text | Government lines in Unicode |
|---|---:|---:|---:|---:|
| Whitespace split | 38.5 / 100.0 | 27.1 / 100.0 | 0.0 / 0.0 | 46.0 / 100.0 |
| Tantivy default (`SimpleTokenizer`, `LowerCaser`) | 47.0 / 57.6 | 39.7 / 51.1 | 0.0 / 0.0 | 64.7 / 75.9 |
| ICU (ICU4X word segmenter, NFKC_Casefold) | 44.6 / 100.0 | 39.6 / 100.0 | 0.0 / 0.0 | 56.3 / 100.0 |
| Meilisearch default (charabia 0.10.0) | 45.3 / 95.7 | 38.1 / 94.5 | 0.0 / 0.0 | 57.2 / 99.8 |
| kannadi-search, spelling only | 44.7 / 100.0 | 44.7 / 100.0 | 56.7 / 99.9 | 57.2 / 99.9 |
| **kannadi-search** | **97.6 / 96.8** | **97.6 / 96.8** | **90.8 / 97.1** | **91.3 / 97.1** |

What the numbers show:

- **Endings are most of the gap.** A Kannada noun takes case, plural and clitic endings, and Kannada writes "is" and "become" (ಇದೆ, ಆಗಿ) joined to the noun: ಮರ appears as ಮರದ, ಮರಗಳಲ್ಲಿ, ಮರವಾಗಿದೆ. Without stemming, a search finds fewer than half of the documents that use the word.
- **Spelling variants cost the other analyzers 5 to 11 points of recall.** kannadi-search scores the same on the variant text as on the clean text. ICU removes joiners through NFKC_Casefold but keeps the nukta, ೦ and Old Kannada letters apart. charabia keeps joiners in tokens and splits words at a ZWNJ (ಕನ್‌ನಡ becomes ಕನ್ and ನಡ).
- **Tantivy's default tokenizer splits words at the virama.** `SimpleTokenizer` keeps runs of alphanumeric characters, and the virama is not alphanumeric, so ಕನ್ನಡ becomes ಕನ and ನಡ. Its recall looks higher because such fragments match many words; its precision is 51% to 76%.
- **Legacy text is invisible to every other analyzer.** On text extracted from legacy-font PDFs they find nothing. kannadi-search converts legacy lines with kannadi's converter and loses half a point against the same lines in Unicode.
- **Precision stays near 97%.** Most of the remaining false matches on Wikipedia are not wrong: the reference lexicon lists some inflected forms as lemmas of their own (ಪ್ರಕಾರವು beside ಪ್ರಕಾರ, ಅನೇಕರು beside ಅನೇಕ), and it files adverbs such as ಪ್ರಮುಖವಾಗಿ under adjectives, which are no query. About 1 in 20 is a real conflation, such as ಹಿಂದು ("Hindu") with ಹಿಂದಕ್ಕೆ ("backwards").

Limits of the measurement:

- Relevance comes from kannada-morph's analyzer with its full lexicon (Alar and Kannada Wikipedia), and the stemmer uses the same analyzer's endings without the lexicon. The baselines do no stemming at all, so their order does not depend on this, but the stemmer's recall is measured against the paradigms it was built from: endings kannada-morph lacks are missing from both.
- Queries are nouns. Verbs are out of scope: ಮಾಡಿದರು does not find ಮಾಡು.
- The government lines' Unicode text is the benchmark's expected text, mostly confirmed by OCR and not yet checked by a reader. That set lives in a private repository, so it is not in this repository and CI does not run it; the Wikipedia sets are in `eval/data` and CI checks their numbers.

Reproduce the table with the steps in [eval/README.md](eval/README.md).

## How it works

[`Analyzer::analyze`](src/lib.rs) runs 4 steps.

1. **Legacy text** ([`legacy`](src/legacy.rs)). A line where at least 40% of the characters, and at least 2, are Windows-1252 characters above ASCII is read as legacy font codes. kannadi picks its layout (Nudi and Baraha, Akruti, Shree-Lipi, Nudi B-Akshara or Kan-Eng) and converts it word by word, so each token keeps the place of its legacy word.
2. **Words**. Text splits at Unicode word boundaries ([UAX #29](https://www.unicode.org/reports/tr29/)), which never fall inside an akshara: vowel signs, the virama, the nukta and joiners stay with their letter. A property test checks that every token starts and ends at an akshara boundary. A word that mixes a number and Kannada gives the number alone: ೨೦೨೬ರಲ್ಲಿ ("in 2026") gives 2026.
3. **Spelling** ([`fold`](src/fold.rs)). Each word becomes its search key: the search form of kannadi's [normalization profile](../../NORMALIZATION.md#search-form) (joiners, the nukta, Old Kannada letters, ೦ typed for ಂ, vowel signs in 2 parts, and the other canonical rules), then a nasal with virama before a consonant of its own group becomes ಂ (ಸಮ್ಬನ್ಧ, ಸಂಬಂಧ), Kannada digits become ASCII digits, and other letters are lowercased.
4. **Endings** ([`stem`](src/stem.rs)). Case, plural and clitic endings, and ಆಗು and ಇರು written joined to a noun, are removed: ಮರಗಳಲ್ಲಿ, ಮರವನ್ನು and ಮರವಾಗಿದೆ all give ಮರ.

The stemmer knows no dictionary. It runs kannada-morph's noun paradigms backwards and is careful in 4 ways:

- A stem keeps at least 2 aksharas: in shorter words an ending is most often part of the word.
- 1,940 words that end like an inflected noun but are stems of their own stay whole, such as ಮೆರವಣಿಗೆ ("procession"), which is not ಮೆರವಣಿ with a dative ending. A word is on this list when Kannada Wikipedia has at least 2 other forms built on it that are not forms of the shorter stem, and those forms are together at least a tenth as frequent as the word.
- When endings give more than 1 stem, the stem Kannada Wikipedia uses most as a word wins: ವ್ಯಕ್ತಿಗೆ ("to the person") reads as ವ್ಯಕ್ತಿ + ಗೆ, as ಕಾಡಿಗೆ reads as ಕಾಡು + ಇಗೆ. 3,785 stems carry the counts for this.
- Otherwise the longest ending wins, and when it leaves 2 different stems, the word stays whole.

A key is a search key and need not be a dictionary headword. A final ು after a consonant becomes a virama, so a loanword and its ು form fold together (ಬ್ಯಾಂಕ್, ಬ್ಯಾಂಕು) and ಕಾಡು's key is ಕಾಡ್. Analyzing a key again gives the same key, so a query typed in the citation form matches.

Use the same analyzer for documents and queries. `Analyzer::new().stemming(false)` gives spelling keys alone, for fields where endings matter; `.legacy(false)` skips legacy detection.

## Data

| File | What | From |
|---|---|---|
| `data/suffixes.tsv` | 1,521 noun endings: stem final, characters to strip, characters to add | kannada-morph's paradigms ([kannada-spell](https://github.com/openkannada/kannada-spell) at 9366c23), by `eval` |
| `data/protected.txt` | 1,940 words the stemmer keeps whole | Kannada Wikipedia word counts of the training articles, by `eval` |
| `data/preferred.tsv` | 3,785 stems with their Wikipedia counts | the same counts, by `eval` |
| `eval/data/` | the Wikipedia evaluation set: sentences, variant sentences, queries and relevant documents | held-out Kannada Wikipedia articles |

The counts come from the Kannada Wikipedia dump of 1 November 2026, split by article as kannada-spell splits it: every 10th article is held out, and the evaluation sentences come only from held-out articles.

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

Token offsets point at the original word, legacy text included, so highlighting works. A test indexes Kannada, variant and legacy documents and checks what each query finds.

## Meilisearch

Meilisearch tokenizes with [charabia](https://github.com/meilisearch/charabia). A Kannada normalizer and segmenter for charabia, self-contained and covering the spelling step, are being prepared as a contribution there. Stemming is not part of it, as charabia stems no language.

## Postgres

[examples/postgres](examples/postgres/README.md) has 2 setups, both run on Postgres 19: an `unaccent` rules file that folds spelling variants inside Postgres full-text search, and a `tsvector` column of kannadi-search keys written by the application, which adds endings and legacy text.

## Speed

On an Apple M1 Pro, in 1 run over the 3,000 Wikipedia sentences repeated 10 times: kannadi-search 15.6 MB/s with stemming and 13.5 MB/s without, charabia 8.8 MB/s, ICU4X 8.7 MB/s, Tantivy's default 4.9 MB/s. The machine was busy with other work, so the rows compare with each other only. Each thread keeps the keys of the last 65,536 distinct words, which is most of the gain on repeated text. Legacy lines are slower, about 0.7 MB/s, because detecting the layout converts each line in every layout.
