<img src="https://raw.githubusercontent.com/openkannada/kannadi/main/docs/images/logo.svg" width="72" height="72" alt="The kannadi logo: the Kannada letter ಕ and its reflection in a mirror, on an indigo tile.">

# kannadi

Convert Kannada documents typed in the old Nudi, Baraha, Nudi B-Akshara, Kan-Eng, Akruti and Shree-Lipi fonts to Unicode, and back. For years Karnataka typed its government orders, budgets and newspapers in fonts that store glyph codes in place of letters, so the text cannot be searched, indexed or read aloud until it is converted. kannadi has been checked on more than 1.8 million glyphs from public Karnataka documents and newspaper pages, and reports every glyph it cannot place.

ಕನ್ನಡಿ (kannaḍi) means mirror.

<p>
  <img src="https://raw.githubusercontent.com/openkannada/kannadi/main/docs/images/editor.png" width="78%" alt="The kannadi editor in a browser after pasting 4 lines from a 2002 Karnataka government page typed in a Baraha font. The text is now Unicode Kannada. 1 glyph the converter could not place and 1 malformed syllable are marked in the third line and listed under Problems.">
  <img src="https://raw.githubusercontent.com/openkannada/kannadi/main/docs/images/editor-phone.png" width="19%" alt="The same 4 lines in the editor on a phone in dark mode, with the paste notice in the glass bar at the top, the Problems panel below the text, and the akshara, word and character counts in the glass bar at the bottom.">
</p>

## Quick start

```sh
brew install openkannada/tap/kannadi
kannadi to-unicode old-orders/ --out converted/   # TXT, HTML, DOCX and PDF files
```

Or paste text into the editor at [openkannada.org/editor](https://openkannada.org/editor/). It runs this library in the browser, so files never leave your device.

## Results

| Check | Result |
|---|---|
| Unicode to Nudi, as typed | 9,029 of 9,047 words from 2002 government pages encode to the exact bytes that were typed |
| Akruti to Unicode | 6 unknown glyphs and 22 malformed syllables in 908,075 glyphs from 35 Acts, Ordinances and budget papers ([method](https://github.com/openkannada/kannadi/blob/main/docs/akruti.md)) |
| Nudi B-Akshara to Unicode | 65 unknown glyphs and 113 malformed syllables in 639,208 glyphs from 22 budget and Assembly papers, many where the PDF lost ಂ ([method](https://github.com/openkannada/kannadi/blob/main/docs/nudi-b.md)) |
| Kan-Eng to Unicode | 1 unknown glyph and 11 malformed syllables in 21,455 glyphs from 45 budget papers ([method](https://github.com/openkannada/kannadi/blob/main/docs/kan-eng.md)) |
| Shree-Lipi to Unicode | 0 unknown glyphs and 20 malformed syllables in 233,346 glyphs from 173 Udayavani pages ([method](https://github.com/openkannada/kannadi/blob/main/docs/shree-lipi.md#the-shree-lipi-table)) |
| Typing Kannada in English letters | 89.19% top-1 and 96.77% top-5 on the Dakshina Kannada test set, against 77.18% top-1 published for IndicXlit; 1.31 ms per keystroke at the median as WebAssembly ([details](https://github.com/openkannada/kannadi/tree/main/crates/kannadi-typing#results)) |
| Transliteration round trip | Exact on 50,000 random strings per CI run; ISO 15919 matches vidyut-lipi on 5,000 random words apart from 1 documented difference |

Each unknown glyph and malformed syllable is kept in the text and listed in the conversion report; each was read against the page, and most are errors in the source. The 1.8 million glyphs are the Akruti, Nudi B-Akshara, Kan-Eng and Shree-Lipi rows above. [corpus/](https://github.com/openkannada/kannadi/tree/main/corpus/) holds 207 lines from 31 government pages of 2002 as a regression set.

## What's inside

| Part | What it does |
|---|---|
| [`kannadi`](https://github.com/openkannada/kannadi/tree/main/crates/kannadi) | The Rust library: legacy font conversion both ways, akshara segmentation, the [Kannada text normalization profile](https://github.com/openkannada/kannadi/blob/main/NORMALIZATION.md), transliteration (ISO 15919, IAST, Baraha), numerals, Old Kannada |
| [`kannadi-cli`](https://github.com/openkannada/kannadi/tree/main/crates/kannadi-cli) | The `kannadi` command: converts files and folders, normalizes text, and types Kannada from English letters ([guide](https://github.com/openkannada/kannadi/blob/main/docs/cli.md)). DOC and RTF input, `normalize` and `type` are on `main` and come in the next release; install from source to use them now |
| [`kannadi-typing`](https://github.com/openkannada/kannadi/tree/main/crates/kannadi-typing) | Ranked Kannada words for loosely typed English letters, learning from the user's picks |
| [`kannadi-wasm`](https://github.com/openkannada/kannadi/tree/main/crates/kannadi-wasm), [`kannadi-typing-wasm`](https://github.com/openkannada/kannadi/tree/main/crates/kannadi-typing-wasm) | WebAssembly bindings for the browser |
| [`kannadi-py`](https://github.com/openkannada/kannadi/tree/main/crates/kannadi-py) | Python bindings |
| [`tools/fontqa`](https://github.com/openkannada/kannadi/tree/main/tools/fontqa) | 2,146 shaping tests for Kannada fonts, with results for 41 fonts |
| [`web/`](https://github.com/openkannada/kannadi/tree/main/web) | A standalone build of the editor page ([guide](https://github.com/openkannada/kannadi/blob/main/docs/editor.md)) |

## Supported fonts

| Layout | Fonts | To Unicode | From Unicode |
|---|---|---|---|
| Nudi | Nudi 01 e to Nudi 05 k, Nudi Akshara, Baraha's BRH fonts | library, command line, editor | library, command line, editor |
| Nudi B-Akshara | NudiB-Akshar, in budget and Assembly papers | library, command line, editor | library |
| Kan-Eng | Kan-EngPraja, Kan-EngDeepa and the other Kan-Eng fonts | library, command line, editor | library |
| Akruti | AkrutiKndPadma and its weights; LangscapeKndPadmini, LLPad and SKPadmini with `--legacy-font` | library, command line, editor | library |
| Shree-Lipi | Shree-Kan-0850 | library, command line, editor | library |

PDF, DOC and RTF files convert to Unicode text. [docs/other-layouts.md](https://github.com/openkannada/kannadi/blob/main/docs/other-layouts.md) lists the fonts that need more public documents before they get a table.

## Install

```sh
# macOS and Linux
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/openkannada/kannadi/releases/latest/download/kannadi-cli-installer.sh | sh

# Windows, in PowerShell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/openkannada/kannadi/releases/latest/download/kannadi-cli-installer.ps1 | iex"

# From source, with Rust 1.89 or later
cargo install --locked --git https://github.com/openkannada/kannadi kannadi-cli
```

Each [release](https://github.com/openkannada/kannadi/releases) has archives for macOS (Apple silicon and Intel), Linux (x86_64 and arm64, glibc and static musl) and Windows (x86_64), each with a SHA-256 checksum, the man pages and the shell completions. The library is the `kannadi` crate; until it is on crates.io, add it from Git:

```toml
kannadi = { git = "https://github.com/openkannada/kannadi" }
```

## Library example

```rust
use kannadi::legacy::{nudi_to_unicode, unicode_to_nudi, Options};
use kannadi::{aksharas, transliterate, Scheme};

assert_eq!(nudi_to_unicode("PÀ£ÀßqÀ", &Options::default()).text, "ಕನ್ನಡ");
assert_eq!(unicode_to_nudi("ಕನ್ನಡ").text, "PÀ£ÀßqÀ");
assert_eq!(aksharas("ಸ್ತ್ರೀಯರು").map(|a| a.text).collect::<Vec<_>>(), ["ಸ್ತ್ರೀ", "ಯ", "ರು"]);
assert_eq!(transliterate("ಕನ್ನಡ ನಾಡು", Scheme::Kannada, Scheme::Iso15919), "kannaḍa nāḍu");
```

[docs/library.md](https://github.com/openkannada/kannadi/blob/main/docs/library.md) covers legacy fonts, segmentation, normalization, transliteration and Old Kannada in full. `cargo run --example demo` shows each feature on 1 sentence.

## Development

```sh
cargo test
PROPTEST_CASES=50000 cargo test --release --test props
cargo clippy --all-targets
```

The minimum supported Rust version is 1.75 for the library and 1.89 for the command-line tool. [CONTRIBUTING.md](https://github.com/openkannada/kannadi/blob/main/CONTRIBUTING.md) explains how to report a file that converts wrongly, the sign-off and releases, and [AGENTS.md](https://github.com/openkannada/kannadi/blob/main/AGENTS.md) has the full contributor guide. [CHANGELOG.md](https://github.com/openkannada/kannadi/blob/main/CHANGELOG.md) lists every change. [docs/prior-art.md](https://github.com/openkannada/kannadi/blob/main/docs/prior-art.md) describes the other converters and libraries.

## Related

- [openkannada.org/editor](https://openkannada.org/editor/) and [openkannada.org/nudi-to-unicode](https://openkannada.org/nudi-to-unicode/): the converter in the browser.
- [openkannada.org/kannada-typing](https://openkannada.org/kannada-typing/): type Kannada with English letters.
- [Neha Sans, Neha Serif](https://github.com/openkannada/noto-kannada-fork) and [Gari](https://github.com/openkannada/gari-font): Kannada fonts to set converted text in.
- [font-health-action](https://github.com/openkannada/font-health-action): fontqa's shaping tests as a GitHub Action.
- [Homebrew tap](https://github.com/openkannada/homebrew-tap): `brew install openkannada/tap/kannadi`.

## License

The code and the glyph tables are under the MIT license or the Apache License 2.0, at your choice: see [LICENSE-MIT](https://github.com/openkannada/kannadi/blob/main/LICENSE-MIT) and [LICENSE-APACHE](https://github.com/openkannada/kannadi/blob/main/LICENSE-APACHE). The docs are under [CC BY 4.0](https://github.com/openkannada/kannadi/blob/main/LICENSES/CC-BY-4.0.txt), and code examples in them may also be used under MIT or Apache 2.0. [REUSE.toml](https://github.com/openkannada/kannadi/blob/main/REUSE.toml) gives the license of every file, and [NOTICE.md](https://github.com/openkannada/kannadi/blob/main/NOTICE.md) lists the third-party material with its credit.

Not covered by these licenses:

- the typing model `crates/kannadi-typing/model/kn.typing` and the Dakshina test sets in `crates/kannadi-typing/eval/`, under [CC BY-SA 4.0](https://github.com/openkannada/kannadi/blob/main/LICENSES/CC-BY-SA-4.0.txt) because they come from Dakshina and Kannada Wikipedia (see the [typing README](https://github.com/openkannada/kannadi/tree/main/crates/kannadi-typing#training-data));
- the excerpts of Karnataka government documents in `corpus/` and in the test fixtures, kept as short excerpts for testing, with their sources (see the [corpus data card](https://github.com/openkannada/kannadi/blob/main/corpus/README.md) and the [fixtures README](https://github.com/openkannada/kannadi/blob/main/crates/kannadi/tests/fixtures/README.md));
- the font names and copyright strings recorded in `tools/fontqa/results/`, which belong to their owners;
- the Noto fonts in `web/src/fonts/`, under the SIL Open Font License (`web/src/fonts/OFL.txt`).

The Shree-Lipi test lines were written for kannadi; no newspaper text is included. The names Open Kannada and kannadi, and the logo, are covered by [TRADEMARKS.md](https://github.com/openkannada/kannadi/blob/main/TRADEMARKS.md). If kannadi helps your project, a mention or a link to [openkannada.org](https://openkannada.org) is appreciated.

Part of [Open Kannada](https://github.com/openkannada), free and open-source tools that make Kannada work properly on computers.
