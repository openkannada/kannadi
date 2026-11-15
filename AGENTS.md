# Working on kannadi

Instructions for anyone changing this repository, human or AI agent. Read this before writing code or docs.

## What this is

A Rust library for Kannada text: akshara segmentation, normalisation, transliteration, numerals, and conversion of legacy Nudi and Baraha font text to Unicode. The main goal is converting legacy documents correctly, with measured accuracy.

## Commands

```sh
cargo test                                              # all tests
PROPTEST_CASES=50000 cargo test --release --test props --test differential
cargo fmt --all
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo +1.75 check -p kannadi --lib                      # minimum supported Rust
cargo build -p kannadi --lib --release --target wasm32-unknown-unknown
cargo run --example nudi -- file.txt                    # convert a legacy text file
```

CI runs all of these. A change is ready when they pass.

## Architecture

```text
crates/kannadi/
  src/chars.rs        character classes for the Kannada block
  src/lookup.rs       longest-prefix lookup shared by the scheme and glyph tables
  src/akshara.rs      segmentation into aksharas (uses chars)
  src/normalize.rs    NFC plus Kannada fixes; Old Kannada modernisation
  src/numerals.rs     Kannada digits
  src/translit/       Kannada to and from ISO 15919, IAST, Baraha
    schemes.rs        the scheme tables, as data
    mod.rs            1 engine for all schemes: romanise and parse
  src/legacy.rs       Nudi and Baraha ANSI to Unicode: a Writer builds the output
                      and the reports; a Syllable decides what it can accept
  data/nudi.tsv       the legacy glyph table, as data
  tests/              golden fixtures, property tests, differential test
docs/                 design notes, corpus plan, prior art
```

Principles:

- **Tables are data and engines are generic.** A new scheme or font layout is a new table, with no new engine code. Keep scheme-specific behaviour as fields on the table.
- **Lossless by default.** Transliteration round-trips Kannada text exactly. When the converter cannot place a glyph, it reports it. Nothing is dropped silently.
- **Small modules with 1 job each.** Shared behaviour goes in 1 place; do not copy code between modules.
- **No abstraction before it earns its place.** Add a trait when a second implementation exists.

## Code rules

- **Idiomatic Rust.** Return `Option` and `Result` from public functions, never `panic!`. A panic is acceptable only on a broken built-in table, which a test catches.
- **Comments.** Write a comment only for what the code cannot say: a Unicode code point or spec reference, a rule that is not obvious, or why a choice was made. Delete comments that restate the code.
- **Doc comments** on public items say what the item guarantees, with an example where it helps.
- **Unicode.** Name characters by code point (U+0CDE) and by glyph (ೞ). Cite the Unicode version when it matters.
- **Third-party code.** Do not copy code or data from other converters. In particular, ascii2unicode is GPL 3.0. Tables are derived from font files and documents; record how in the table's header.

## Tests

- Every behaviour change gets a test.
- For each new rule, remove the rule, confirm a test fails, then restore it.
- Use property tests for round trips and invariants. Add a golden fixture for real text.
- Fixtures from public documents are short excerpts. Record the source URL with each.

## Pull requests

- Branch from `main`. Never push to `main`.
- Commits are signed. Commit messages and PR bodies carry no AI attribution lines.
- The PR body says what changed, why, how it was verified, and what was refactored.
- Every PR gets an independent review covering bugs and design quality. Fix the findings, then merge with a squash when CI is green.

## Writing

This applies to the README, docs, comments, commit messages and PR text.

- Plain, short sentences. No hype words.
- No em dashes, no rhetorical questions, no "X, not Y" constructions.
- Digits for numbers. Sentence-case headings.
- Say what is true and what is not done.
