# Working on kannadi

Instructions for anyone changing this repository, human or AI agent. Read this before writing code or docs.

## What this is

A Rust library for Kannada text: akshara segmentation, normalization, transliteration, numerals, and conversion of legacy Nudi, Baraha, Nudi B-Akshara, Kan-Eng, Akruti and Shree-Lipi font text to Unicode. The main goal is converting legacy documents correctly, with measured accuracy.

## Commands

```sh
cargo test                                              # all tests
PROPTEST_CASES=50000 cargo test --release --test props --test differential
cargo fmt --all
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo +1.75 check -p kannadi --lib                      # minimum supported Rust
web/build.sh && node web/test/smoke.mjs                 # WebAssembly build and editor page
crates/kannadi-typing-wasm/build.sh && node crates/kannadi-typing-wasm/test/smoke.mjs  # typing engine in WebAssembly
cargo run --release -p kannadi-typing --features train --bin kannadi-typing-train -- \
  eval --model crates/kannadi-typing/model/kn.typing --test crates/kannadi-typing/eval/dakshina-kn-test.tsv
node --test web/test/units.mjs                          # page modules
python3 web/serve.py 8000                               # serve web/dist locally
cargo run --example nudi -- file.txt                    # convert a legacy text file
cargo run -p kannadi-cli -- to-unicode docs/ --out out/ # batch convert
cd tools/fontqa && uv run pytest && uv run ruff check . ../glyphdraft # font shaping tests (Python)
```

CI runs these with `--locked`, apart from the example. A change is ready when they pass. `web/build.sh` needs the `wasm32-unknown-unknown` target and `wasm-bindgen-cli` at the `wasm-bindgen` version in `Cargo.lock`.

## Architecture

```text
crates/kannadi/
  src/lib.rs          public API and re-exports
  src/chars.rs        character classes for the Kannada block
  src/lookup.rs       longest-prefix lookup shared by the scheme and glyph tables
  src/akshara.rs      segmentation into aksharas (uses chars)
  src/normalize.rs    the normalization profile (NORMALIZATION.md): canonical, legacy
                      and search forms, counts per rule; Old Kannada modernization
  src/numerals.rs     Kannada digits: conversion, formatting, parsing
  src/phonetic.rs     sound-alike keys for Kannada words and loose Latin spellings
  src/typos.rs        well-formed spellings that are often typing mistakes
  src/translit/       Kannada to and from ISO 15919, IAST, Baraha
    schemes.rs        the scheme tables, as data
    mod.rs            1 engine for all schemes: romanize and parse
  src/legacy/         legacy font text to and from Unicode, 1 table per Layout
    table.rs          parses each layout's table once for both directions
    decode.rs         to Unicode: a Writer builds the output and the reports;
                      a Syllable decides what it can accept
    encode.rs         from Unicode: each akshara written in storage order
    fonts.rs          font names: each font's layout, legacy fonts with no table,
                      Kannada digits
  data/nudi.tsv       the Nudi and Baraha ANSI glyph table, as data
  data/akruti.tsv     the Akruti glyph table
  data/shree-lipi.tsv the Shree-Lipi glyph table
  data/nudi-b.tsv     the Nudi B-Akshara glyph table
  data/kan-eng.tsv    the Kan-Eng glyph table
  tests/              golden fixtures, property tests, differential test
  examples/           demo, and a command-line converter for legacy files
crates/kannadi-cli/   the `kannadi` binary: batch conversion of files and folders
  src/files.rs        which files to convert and where each one goes
  src/convert.rs      1 piece of text in either direction, with its issues
  src/report.rs       the per-file report
  src/normalize.rs    `kannadi normalize`: text files or standard input, line by line
  src/fonts.rs        names added with --legacy-font, and font-family lists
  src/edits.rs        rewriting a document by byte ranges
  src/txt.rs          plain text files
  src/html/           HTML: a small tokenizer, CSS font rules, character references
  src/docx/           DOCX: styles, and paragraphs, runs and text found by position
  src/pdf/            PDF text: a page reader, font judging, widths, ToUnicode
                      maps and Differences lists
crates/kannadi-typing/  typing Kannada in English letters: ranked words, learning from picks
  src/units.rs        Kannada words as sound units (the inherent a is a unit)
  src/roman.rs        the Latin letters the model reads, and letter chunks
  src/lm.rs           the n-gram model over (letters, unit) pairs, as a trie
  src/decode.rs       beam search, through the lexicon automaton or without it
  src/complete.rs     the commonest lexicon word that starts with a prefix
  src/rules.rs        strict Baraha and ITRANS readings
  src/user.rs         the user's picks, in memory, exported as text
  src/model.rs        the model file format and the ranking weights
  src/train/          alignment (EM), Kneser-Ney estimation, data readers,
                      evaluation and tuning; behind the `train` feature
  model/kn.typing     the trained model (CC BY-SA 4.0); train.sh rebuilds it
  eval/               the Dakshina Kannada dev and test lexicons (CC BY-SA 4.0)
crates/kannadi-typing-wasm/  wasm-bindgen bindings for the typing engine; build.sh,
                      test/smoke.mjs (checks and per-keystroke timing)
crates/kannadi-wasm/  wasm-bindgen bindings for the page; offsets in UTF-16 code units
web/
  src/                the static editor page: HTML, CSS, fonts (OFL), JavaScript
    js/main.js        wires the page: toolbar, files, paste, shortcuts, autosave
    js/editor.js      plain-text contenteditable with its own undo history
    js/issues.js      unknown glyphs and malformed syllables: marks, list, fixes
    js/compose.js     phonetic typing in Baraha and ISO 15919
    js/files.js       open TXT, HTML and DOCX; save TXT, HTML and DOCX
    js/docx.js        DOCX text and run fonts; zip.js reads and writes ZIP
    js/text.js        pure helpers: legacy detection, diffs, anchors
    js/kannadi.js     wrapper over the WebAssembly bindings
  site/pages.mjs      the content of each page: title, definition, how-to, FAQ
  site/build.mjs      writes the pages, JSON-LD, robots.txt, llms.txt and the sitemap
  site/og.mjs         renders the Open Graph cards in src/og with Chrome
  build.sh            builds the WebAssembly package and the pages into web/dist
  wasm-bindgen-version.sh  the wasm-bindgen version in Cargo.lock, for build.sh and CI
  serve.py            local server for web/dist
  DESIGN.md           why the page looks as it does: world, tokens, contrast rules
  test/smoke.mjs      checks the built package in Node
  test/units.mjs      tests the page modules in Node, with no WebAssembly
tools/fontqa/         Python: HarfBuzz shaping tests for Kannada fonts, results,
                      and fixes for font tables; see its README
tools/glyphdraft/     FontForge Python: drafts U+0CDC, U+0CDD, U+0CF3 into an .sfd
corpus/               benchmark lines from real documents, with a data card
normalization/        test vectors for NORMALIZATION.md (TSV, and JSON written from it
                      by tests/normalization.rs), and the HarfBuzz rendering check
docs/                 design notes, corpus plan, prior art
```

Principles:

- **Tables are data and engines are generic.** Scheme spellings and glyph layouts live in tables. A new transliteration scheme needs a table and an entry in the `Scheme` enum. A new legacy font layout needs a table in `data/`, a variant in `Layout` with its name, and its file in `table.rs`. Keep scheme-specific behavior as fields on the table.
- **Lossless by default.** Transliteration round-trips Kannada text exactly. When the converter cannot place a glyph, it reports it. Nothing is dropped silently, except spacing glyphs, which carry no text, and spaces a table marks as typed before a sign.
- **Small modules with 1 job each.** Shared behavior goes in 1 place; do not copy code between modules.
- **No abstraction before it earns its place.** Add a trait when a second implementation exists.

## Code rules

- **Idiomatic Rust.** Report failure with `Option`, `Result` or a report value, and never panic on user input. A panic is acceptable only on a broken built-in table, which a test catches.
- **Comments.** Write a comment only for what the code cannot say: a Unicode code point or spec reference, a rule that is not obvious, or why a choice was made. Delete comments that restate the code.
- **Doc comments** on public items say what the item guarantees, with an example where it helps.
- **Unicode.** Name characters by code point (U+0CDE) and by glyph (ೞ). Cite the Unicode version when it matters.
- **Third-party code.** Do not copy code or data from other converters. In particular, ascii2unicode is GPL 3.0. Tables are derived from font files and documents; record how in the table's header.

## Editor page

- The page has no build tool, framework or third-party script. Keep it that way unless a need appears.
- Kannada rules (conversion, syllable checks, fixes, transliteration) live in the crate and reach the page through `kannadi-wasm`. JavaScript handles the DOM, files and events.
- Change the document through `DocEditor.replace`, so the edit is reported, marked and undoable.
- Keep `web/src/js/text.js`, `docx.js`, `zip.js` and `compose.js` free of DOM access, so `units.mjs` can test them in Node.
- No tracking, no cookies and no network requests beyond the page's own files. The Content-Security-Policy in `index.html` enforces this.
- Build DOM nodes with `textContent`, never `innerHTML`, since the report shows user text.
- Page copy follows the writing rules below. Do not claim a feature before it ships.
- Page content lives in `web/site/pages.mjs`. The visible FAQ and the FAQPage JSON-LD come from the same entries, so they cannot drift. Rerun `node web/site/og.mjs` after changing a page title.

## Tests

- Every behavior change gets a test.
- For each new rule, remove the rule, confirm a test fails, then restore it.
- Use property tests for round trips and invariants. Add a golden fixture for real text.
- Fixtures from public documents are short excerpts. Record the source URL with each. Do not add newspaper or other commercial text: write the lines yourself and encode them with the layout's table, as `tests/fixtures/shree-lipi.tsv` does.
- Every file needs a license in `REUSE.toml` or an SPDX header. Check with `uvx reuse@6.3.0 lint`.

## Pull requests

- Branch from `main`. Never push to `main`.
- Commits are signed and signed off (`git commit -S -s`): every commit needs a `Signed-off-by` line under the [Developer Certificate of Origin](https://developercertificate.org/). Commit messages and PR bodies carry no AI attribution lines.
- The PR body says what changed, why, how it was verified, and what was refactored.
- Every PR gets an independent review covering bugs and design quality. Fix the findings, then merge with a squash when CI is green.

## Writing

This applies to the README, docs, comments, commit messages and PR text.

- Plain, short sentences. No hype words.
- No em dashes, no rhetorical questions, no "X, not Y" constructions.
- Digits for numbers. Sentence-case headings.
- Say what is true and what is not done.
