# kannadi-search evaluation

`kannadi-search-eval` builds the stemmer's data files and scores search analyzers. It is a separate crate, outside the workspace: it depends on kannada-morph from [kannada-spell](https://github.com/openkannada/kannada-spell), which is not public yet, and on the baselines (charabia, ICU4X, Tantivy), which only scoring needs. CI does not build it. CI does run `tests/recall.rs` in kannadi-search, which scores the analyzer on the committed Wikipedia set.

## Inputs

- The Kannada Wikipedia dump of 1 November 2026: `knwiki-20261101-pages-articles.xml.bz2` from dumps.wikimedia.org.
- kannada-spell's word counts of that dump (`kspell corpus`, which writes `data/build/wiki-train.tsv` and `wiki-test.tsv`) and its lexicon (`data/build/lexicon.tsv`). Every 10th article goes to the test counts; this tool holds out the same articles.
- The Karnataka government lines of [legacy-encodings](https://github.com/openkannada/legacy-encodings) (`benchmark/lines/`), at b3f4f34.

## Steps

```sh
cd crates/kannadi-search/eval
SPELL=../../../../kannada-spell          # a kannada-spell checkout with data/build
E=target/release/kannadi-search-eval
cargo build --release

# Data files of the stemmer, in this order: each step reads the one before.
$E rules ../data/suffixes.tsv --predicates
cargo build --release
$E prefer $SPELL/data/build/wiki-train.tsv ../data/preferred.tsv
cargo build --release
$E protect $SPELL/data/build/wiki-train.tsv ../data/protected.txt
cargo build --release

# The Wikipedia sets: 2 disjoint samples of 3,000 held-out sentences, each also with
# variants. wiki-dev is for reading errors (`errors`), wiki is the test set, only scored.
# The last argument keeps every held-out sentence, for `speed`.
bzcat knwiki-20261101-pages-articles.xml.bz2 | $E wiki 3000 data all-sentences.tsv
for set in wiki-dev wiki; do
  $E gold $SPELL/data/build/lexicon.tsv data/$set-docs.tsv 1000 data/$set-queries.tsv $SPELL/data/build/wiki-train.tsv
done
$E score data/wiki-queries.tsv data/wiki-docs.tsv
$E score data/wiki-queries.tsv data/wiki-variant-docs.tsv
$E errors data/wiki-dev-queries.tsv data/wiki-dev-docs.tsv
$E speed all-sentences.tsv

# The government set, kept out of the repository.
$E gov ../../../../legacy-encodings/benchmark/lines gov
$E gold $SPELL/data/build/lexicon.tsv gov/gov-docs.tsv 1000 gov/gov-queries.tsv $SPELL/data/build/wiki-train.tsv
$E score gov/gov-queries.tsv gov/gov-legacy-docs.tsv
$E score gov/gov-queries.tsv gov/gov-docs.tsv
```

`errors` lists the words behind false matches and misses, `why` shows the Wikipedia evidence for a protected word, `speed` times 1 pass of each analyzer on its own thread, and `profile` times kannadi-search's steps.

## Relevance

A document is relevant to a query noun when 1 of its words is the noun, or kannada-morph's analyzer, with its full lexicon, reads the word as a form of it: case, plural, clitics, and the predicates Kannada writes joined to a noun (ಮರವಾಗಿದೆ is relevant to ಮರ). The analyzer reads the clean text in the search form of kannadi's normalization profile, so a variant spelling already in Wikipedia counts as its word.

Queries are nouns of 2 or more aksharas that are relevant to 2 or more documents and appear as a word at least 10 times in the Wikipedia training articles, 1,000 of them picked at random with a fixed seed. 2 kinds of lexicon entries are left out: stems no one types alone (ಮತ್ of ಮತ್ತು), by the count, and lemmas that are themselves a form of another lexicon entry (the lexicon lists ಮರವು beside ಮರ, and ಅನೇಕರು beside ಅನೇಕ), by the analyzer and by kannada-morph's endings. These filters apply to every analyzer alike, but the second one helps a stemmer more: such a lemma is where a stemmer would merge 2 entries and an analyzer without stemming loses nothing.

A system finds a document when the document's tokens include every token of the query, as an AND search does. Recall and precision are summed over all queries.

## Variant spellings

`wiki-variant-docs.tsv` is `wiki-docs.tsv` with these changes, drawn with a fixed seed ([src/variants.rs](src/variants.rs)):

| Variant | How often |
|---|---|
| ZWNJ after a word-final virama, as Kannada Wikipedia often writes ಆಫ್‌ | every time |
| ZWNJ or ZWJ between a virama and the next consonant | 5% each |
| ೦ (Kannada zero) for ಂ, as legacy converters and OCR write it | 30% |
| a nasal with virama for ಂ before a consonant of its group (ಸಮ್ಬನ್ಧ) | 30% |
| ೞ for ಳ, ಱ for ರ | 5% |
| a nukta on ಜ and ಫ | 30% |
| ೀ, ೇ, ೊ and ೋ typed as 2 signs | 30% |
| Kannada digits for a number | 50% |

## Licenses

The sentences in `data/` are from Kannada Wikipedia, under CC BY-SA 4.0. The query lists were made with kannada-morph's lexicon, which contains information from Alar by V. Krishna (https://alar.ink), made available under the Open Database License 1.0.
