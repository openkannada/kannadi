# Working on kannadi

Instructions for anyone changing this repository, human or AI agent. Read this before writing code or docs.

## What this is

A Rust library for Kannada text: akshara segmentation, normalization, transliteration, numerals, and conversion of legacy Nudi and Baraha font text to Unicode. The main goal is converting legacy documents correctly, with measured accuracy.

## Commands

```sh
cargo test                                              # all tests
PROPTEST_CASES=50000 cargo test --release --test props --test differential
cargo fmt --all
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
cargo +1.75 check -p kannadi --lib                      # minimum supported Rust
web/build.sh && node web/test/smoke.mjs                 # WebAssembly build and converter page
cargo run --example nudi -- file.txt                    # convert a legacy text file
```

CI runs these with `--locked`, apart from the example. A change is ready when they pass. `web/build.sh` needs the `wasm32-unknown-unknown` target and `wasm-bindgen-cli` at the `wasm-bindgen` version in `Cargo.lock`.

## Architecture

```text
crates/kannadi/
  src/lib.rs          public API and re-exports
  src/chars.rs        character classes for the Kannada block
  src/lookup.rs       longest-prefix lookup shared by the scheme and glyph tables
  src/akshara.rs      segmentation into aksharas (uses chars)
  src/normalize.rs    NFC plus Kannada fixes; Old Kannada modernization
  src/numerals.rs     Kannada digits: conversion, formatting, parsing
  src/translit/       Kannada to and from ISO 15919, IAST, Baraha
    schemes.rs        the scheme tables, as data
    mod.rs            1 engine for all schemes: romanize and parse
  src/legacy/         Nudi and Baraha ANSI to and from Unicode
    table.rs          parses data/nudi.tsv once for both directions
    decode.rs         to Unicode: a Writer builds the output and the reports;
                      a Syllable decides what it can accept
    encode.rs         from Unicode: each akshara written in storage order
  data/nudi.tsv       the legacy glyph table, as data
  tests/              golden fixtures, property tests, differential test
  examples/           demo, and a command-line converter for legacy files
crates/kannadi-wasm/  wasm-bindgen bindings for the page; offsets in UTF-16 code units
web/
  src/                the static converter page: HTML, CSS, 1 JavaScript module
  build.sh            builds the WebAssembly package and the page into web/dist
  wasm-bindgen-version.sh  the wasm-bindgen version in Cargo.lock, for build.sh and CI
  test/smoke.mjs      checks the built package in Node
docs/                 design notes, corpus plan, prior art
```

Principles:

- **Tables are data and engines are generic.** Scheme spellings and glyph layouts live in tables. A new transliteration scheme needs a table and an entry in the `Scheme` enum. A new legacy font layout needs a table; the converter currently loads only `nudi.tsv`. Keep scheme-specific behavior as fields on the table.
- **Lossless by default.** Transliteration round-trips Kannada text exactly. When the converter cannot place a glyph, it reports it. Nothing is dropped silently, except spacing glyphs, which carry no text.
- **Small modules with 1 job each.** Shared behavior goes in 1 place; do not copy code between modules.
- **No abstraction before it earns its place.** Add a trait when a second implementation exists.

## Code rules

- **Idiomatic Rust.** Report failure with `Option`, `Result` or a report value, and never panic on user input. A panic is acceptable only on a broken built-in table, which a test catches.
- **Comments.** Write a comment only for what the code cannot say: a Unicode code point or spec reference, a rule that is not obvious, or why a choice was made. Delete comments that restate the code.
- **Doc comments** on public items say what the item guarantees, with an example where it helps.
- **Unicode.** Name characters by code point (U+0CDE) and by glyph (ೞ). Cite the Unicode version when it matters.
- **Third-party code.** Do not copy code or data from other converters. In particular, ascii2unicode is GPL 3.0. Tables are derived from font files and documents; record how in the table's header.

## Converter page

- The page has no build tool, framework or third-party script. Keep it that way unless a need appears.
- No tracking, no cookies and no network requests beyond the page's own files. The Content-Security-Policy in `index.html` enforces this.
- Build DOM nodes with `textContent`, never `innerHTML`, since the report shows user text.
- Page copy follows the writing rules below. Do not claim a feature, such as Unicode to Nudi, before it ships.

## Tests

- Every behavior change gets a test.
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
