# kannadi

Tools for Kannada text in Rust. The goal is to convert documents typed in old Nudi and Baraha fonts to Unicode, and back, with checked accuracy.

ಕನ್ನಡಿ (kannaḍi) means mirror.

## Why

For years, Kannada text in Karnataka was typed with Nudi and Baraha fonts that store glyph codes in place of letters. Government orders, court records and newspaper archives from that time cannot be searched, indexed or read aloud until they are converted. The open converters available today disagree with each other, and none publishes a verified test set.

## Status

Version 0.1, in development.

| Part | State |
|---|---|
| Akshara segmentation | Done |
| Normalization and Old Kannada modernization | Done |
| Transliteration: ISO 15919, IAST, Baraha phonetic | Done |
| Kannada numerals | Done |
| Nudi and Baraha to Unicode | Done, waiting on a native reader's check of 100 sample lines |
| Unicode to Nudi and Baraha | Done |
| Test corpus | 207 lines from 31 government pages of 2002 in [corpus/](corpus/); a Kannada reader's check is pending |
| Editor page that runs in the browser, with no upload | Built, not deployed: [web/](web/) |
| Command-line tool for TXT, HTML, DOCX and PDF | Done; PDF to Unicode only |
| Shaping tests for Kannada fonts, with results for 41 fonts | Done: [tools/fontqa](tools/fontqa) |

## Example

```rust
use kannadi::{aksharas, fix_zero_as_anusvara, modernize, normalize, numerals, transliterate, Scheme};

// Aksharas: the units a reader sees as 1 letter.
let parts: Vec<&str> = aksharas("ಸ್ತ್ರೀಯರು").map(|a| a.text).collect();
assert_eq!(parts, ["ಸ್ತ್ರೀ", "ಯ", "ರು"]);

// Normalization fixes text that looks right but compares wrong.
assert_eq!(normalize("ಕಿ\u{0CD5}ರ್ತಿ"), "ಕೀರ್ತಿ"); // vowel sign typed in 2 parts

// For converted or scanned text: a digit zero typed for an anusvara.
assert_eq!(fix_zero_as_anusvara("ಬೆ೦ಗಳೂರು"), "ಬೆಂಗಳೂರು");

// Old Kannada to modern spelling, for search.
assert_eq!(modernize("ಅಱಿವು ಬಾೞ್"), "ಅರಿವು ಬಾಳ್");

// Transliteration, both ways.
assert_eq!(transliterate("ಕನ್ನಡ ನಾಡು", Scheme::Kannada, Scheme::Iso15919), "kannaḍa nāḍu");
assert_eq!(transliterate("kannaDa nADu", Scheme::Baraha, Scheme::Kannada), "ಕನ್ನಡ ನಾಡು");

// Numerals.
assert_eq!(numerals::format(1956), "೧೯೫೬");
```

Run `cargo run --example demo` to see each feature on 1 sentence.

## Legacy fonts

Nudi (up to version 5) and Baraha's ANSI fonts store glyph codes in place of letters, so the text reads as `PÀ£ÀßqÀ` outside the font. `legacy::nudi_to_unicode` converts it:

```rust
use kannadi::legacy::{nudi_to_unicode, Options};

let converted = nudi_to_unicode("PÀ£ÀßqÀ", &Options::default());
assert_eq!(converted.text, "ಕನ್ನಡ");
assert!(converted.unknown.is_empty());
```

- **How the table was built.** The glyph table in [nudi.tsv](crates/kannadi/data/nudi.tsv) comes from the Nudi font files. Each glyph was rendered and matched against the Unicode version of the same typeface, which HarfBuzz labels. The table was then checked against typed letter charts and Karnataka government pages from 2002.
- **Nothing is dropped silently.** Glyphs the converter cannot place are kept in the output and listed in `unknown`, with their byte offsets. Syllables that cannot be valid Kannada are listed in `malformed`: a mark with no letter, 2 vowel signs on 1 consonant, a sign on a vowel letter, or a consonant shape typed without its sign. Spacing glyphs carry no text and are the only glyphs dropped.
- **Digits.** The `k` variants of the Nudi fonts draw Kannada digits at the ASCII digit positions. For text in those fonts, set `Options::kannada_digits`.
- **Typed zeros.** A zero typed between 2 Kannada letters in place of an anusvara (ಪ್ರಾರ0ಭ) becomes ಂ by default.
- **Limits.**
  - Some glyphs look alike: the ೂ in ವೊ and ಮೊ, the ು in ವು and the tail of ಮ, and the subscript ಜ and ಞ. A document that used the wrong key converts to what was typed.
  - English text set in the same font is converted too, so pass only the legacy-font runs.

Run `cargo run --example nudi -- file.txt` to convert a Windows-1252 text file.

`legacy::unicode_to_nudi` goes the other way, for documents that must still be printed in a legacy font:

```rust
use kannadi::legacy::unicode_to_nudi;

let encoded = unicode_to_nudi("ಕನ್ನಡ");
assert_eq!(encoded.text, "PÀ£ÀßqÀ");
assert!(encoded.is_clean());
```

- **Storage order.** Each akshara is written the way typists stored it: the consonant shape, the vowel sign, the subscripts, ೃ and the length marks ೕ and ೖ, the arkavattu, then ಂ or ಃ. Where the layout has 2 glyphs for 1 sign, such as the ು of ವು or a second subscript ರ, the table marks which one the font uses there. On 9,047 words from 35 Karnataka government pages from 2002, 9,029 encode to the exact bytes that were typed. The rest differ in typing habits: a zero for ಂ, an extra spacing glyph, or ರ್ typed as its own syllable.
- **Round trip.** For well-formed Kannada, converting to Nudi and back gives the same text. A property test checks this.
- **Unencodable characters.** Characters the layout has no glyph for, such as ಌ, ೄ, ೝ, the nukta, ZWJ, ZWNJ, Latin letters and spaces with no Windows-1252 byte, are kept in the output and listed in `unencodable` with their byte offsets. Aksharas that are not well formed, such as 2 vowel signs on 1 consonant, are listed in `malformed`. Kannada digits are written as ASCII digits; set the text in a `k` font to show Kannada digits.

## Editor page

[web/](web/) is a static Kannada editor that runs in the browser. The crate runs as WebAssembly through the bindings in [crates/kannadi-wasm](crates/kannadi-wasm), so text never leaves the device.

- Paste text, or open a TXT, DOCX or HTML file. Nudi and Baraha text is converted to Unicode. In DOCX and HTML files, only text set in a Nudi or Baraha font is converted.
- Unknown glyphs and malformed syllables are marked in the text and listed beside it. Selecting one offers checked fixes, or a replacement typed in Baraha letters.
- Type Kannada with Baraha or ISO 15919 letters.
- Normalize, modernize Old Kannada letters, and count aksharas and words.
- Save as DOCX, TXT or HTML. The last document is kept in the browser's local storage.

Build and run it locally. It needs the `wasm32-unknown-unknown` target and the `wasm-bindgen` command at the version in `Cargo.lock`:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130 --locked
web/build.sh
python3 web/serve.py 8000
```

`web/dist` is a plain folder of static files that any static host can serve. `web/serve.py` is only for local testing: the standard `python3 -m http.server` drops some of the page's parallel requests. `node --test web/test/units.mjs` tests the page modules, and `node web/test/smoke.mjs` checks the built package.

## Command-line tool

The `kannadi` binary in `crates/kannadi-cli` converts files and folders in both directions:

```sh
cargo install --path crates/kannadi-cli
kannadi to-unicode old-orders/ --out converted/
kannadi to-nudi letter.txt --out for-printing/ --strict
```

- A file goes to `OUT/<file name>`. A folder goes to `OUT/<folder name>/`, with its subfolders kept. Hidden files, files already under `OUT`, and symbolic links to folders are skipped. The tool stops before writing anything if an output would overwrite any input or 2 inputs would share an output; paths are compared with links and `..` resolved, ignoring case.
- Text files with a UTF-16 or UTF-8 byte order mark, or valid UTF-8, are read as such. Other legacy text files are read as Windows-1252. Unicode output is UTF-8. Nudi output is Windows-1252, with `?` for each character that has no glyph; those characters are in the report.
- When a file has unknown glyphs, malformed syllables or unencodable characters, the tool writes `FILE.report.tsv` next to the output: the line, the problem, and up to 30 characters of Unicode text on each side. Rows are in line order. A clean run removes an old report.
- `--strict` exits with status 1 if any file has a problem. Errors, such as an unreadable file or folder, exit with status 2; the other files are still converted.
- `to-unicode` takes `--kannada-digits` and `--keep-zeros`, the same settings as `legacy::Options`.
- **HTML.** Only text set in a Nudi or Baraha ANSI font is converted to Unicode. The font of each piece of text comes from its `style` attribute, then `<style>` rules for tags and classes such as `td.xl24` or `.font12` (the most specific, then the last), then `<font face>`, inherited from enclosing elements. End tags HTML lets authors leave out, such as `</td>` and `</p>`, are closed where a browser would close them. Legacy font names are replaced with `--font` (Noto Sans Kannada by default). `to-nudi` converts Kannada text outside legacy fonts and wraps it in a span set in `--font` (Nudi 01 e by default); the rest of the text is copied as it was. A page that declares a Windows-1252 or ISO-8859-1 charset is read that way. Output is UTF-8, and the charset declaration says so. Text in `<title>`, `<textarea>` and attribute values is not converted, and a syllable split across tags is converted piece by piece, so it shows as malformed in the report. More complex selectors and the `font` shorthand are not read.
- **DOCX.** A run's font comes from its own `w:rFonts`, its character style, its paragraph style (or the default paragraph style), or the document defaults, following `basedOn`. Only runs whose Latin font is a Nudi or Baraha ANSI font are converted to Unicode. Their fonts are set to `--font`. Word often splits a word across runs; text is moved between neighboring runs so no syllable is split. `to-nudi` moves each Kannada run of text into a new run set in `--font`, with the rest of the run's formatting. The body, headers, footers, footnotes, endnotes and comments are converted; every other part of the package is copied unchanged. Report rows name the part and paragraph; the fallback copy of a text box is converted but not reported twice. Where a run sets `w:b`, `w:i` or `w:sz` but not their complex-script copies, these are added, since Word formats Kannada with `w:bCs`, `w:iCs` and `w:szCs`; `to-nudi` copies them the other way. Tracked deletions are converted too. Not done: bold, italic and sizes that come from styles, symbols inserted with `w:sym`, fonts of paragraph marks and styles, and parts written with a namespace prefix other than `w:`.
- **PDF.** `to-unicode` reads PDF files and writes their text to `NAME.pdf.txt`, with text in legacy fonts converted. Legacy fonts are found by name, or by their text when the PDF hides the name. A font whose codes were renumbered is read through its ToUnicode map when the map gives the codes back. Other renumbered fonts, and legacy fonts in other layouts such as Akruti, are reported and left out. [docs/pdf.md](docs/pdf.md) has the method and results on public PDFs.
- **Legacy font names.** The tool knows the ANSI Nudi fonts (Nudi 01 e to Nudi 05 k, Nudi Akshara, Nudi B-Akshara) and Baraha's BRH fonts for Kannada. `--legacy-font NAME` adds another.

## Segmentation

`aksharas(text)` splits text into aksharas and other segments. Joining the segments gives back the input exactly. The rules cover conjuncts of any length, nukta, ZWJ and ZWNJ after a virama, dead consonants at the end of a word, and Old Kannada letters. They are written out in [akshara.rs](crates/kannadi/src/akshara.rs).

The legacy converter uses segmentation to check its own output.

## Normalization

`normalize(text)` applies Unicode NFC, then 2 Kannada fixes:

1. ಉ + ಾ becomes ಊ, and ಋ + ಾ becomes ೠ. They look the same, and Unicode says to use the single letter.
2. A mark typed twice in a row (್್, ಂಂ) is kept once.

`fix_zero_as_anusvara(text)` turns a Kannada zero (೦) written right after a letter into an anusvara (ಂ). Legacy fonts and OCR often mix the 2 up. It is a separate step because it also changes a number written straight after a word, such as ಕೊಠಡಿ೦೧.

`modernize(text)` maps ೞ to ಳ, ಱ to ರ and ೝ to ನ್. When a consonant follows ೝ, a ZWNJ keeps the ನ್ from joining it.

## Transliteration

| Kannada | ISO 15919 | IAST | Baraha |
|---|---|---|---|
| ಕನ್ನಡ | kannaḍa | kannaḍa | kannaDa |
| ಬೆಂಗಳೂರು | beṁgaḷūru | beṃgaḷūru | beMgaLUru |
| ಸಂಸ್ಕೃತ | saṁskr̥ta | saṃskṛta | saMsk~ruta |
| ಏಳು | ēḷu | ēḷu | ELu |
| ಅಱಿವು | aṟivu | aṟivu | arxivu |
| ಮೞೆ | maḻe | maḻe | maLxe |

The full list of tested words is in [transliteration.tsv](crates/kannadi/tests/fixtures/transliteration.tsv).

- **Exact round trips.** Kannada to Latin and back gives the same code points for Kannada text, including spaces, punctuation and joiners. The 1 exception is digits, below. Property tests check 2,000 random strings per run, and 50,000 in CI.
- **Separators.** When 2 letters would otherwise read as 1, a separator goes between them: `k:ha` for ಕ್ಹ and `ka:i` for ಕಇ in ISO 15919 and IAST. Baraha uses `~h` for ಹ after a consonant (`bakkiMg~hAm`), as Baraha's own documentation does, and `_` elsewhere.
- **Short and long e and o.** ಎ/ಏ and ಒ/ಓ stay apart in all 3 schemes.
- **IAST for Kannada.** Plain IAST was made for Sanskrit and has no letters for short e, short o, ಱ, ಳ or ೞ. This crate uses the ISO 15919 letters for those, and writes vocalic l as `l̥` because `ḷ` is ಳ.
- **Input.** Baraha accepts its alternative spellings (`aa` or `A`). ISO 15919 and IAST input is read without regard to case or Unicode composition.
- **Digits.** Kannada digits become ASCII digits in Latin output. Use `numerals::to_kannada_digits` to go the other way.
- **Pass-through.** Characters a scheme has no spelling for are copied as they are and still round-trip. Examples: ZWNJ in ISO 15919, nakaara pollu (ೝ), and nukta forms other than ಜ಼ and ಫ಼ in ISO 15919 and IAST.
- **Escapes.** A literal character that a scheme would read as part of a token is written with a backslash: `\'` for an apostrophe in ISO 15919 and IAST, `\~` `\@` `\^` in Baraha, and `\\` for a backslash. A `:` or `_` is a separator only between 2 letters, so `samaya: 10:30` stays as written.
- **Limits.** Latin to Kannada converts every Latin letter it can read, including English words.
- **IAST from Sanskrit sources.** In Sanskrit IAST, `e` and `o` are long vowels. This crate reads them as short ಎ and ಒ, so Sanskrit text in IAST comes out with short vowels.
- **Baraha `R`.** Current Baraha documentation uses `R` and `rx` for ಱ, and `~ru` for ಋ. Older Baraha text that writes ಋ as `Ru` (as in `kRuShNa`) reads as ಱು.

ISO 15919 output is cross-checked against [vidyut-lipi](https://github.com/ambuda-org/vidyut) on 5,000 random words per test run. The 2 crates differ on dead ಱ only: vidyut-lipi writes `r̆`, after the Marathi eyelash ra, and this crate writes `ṟ`. Nukta forms other than ಜ಼ and ಫ಼ are left out of the comparison.

## Old Kannada

ಱ, ೞ (which Unicode names "KANNADA LETTER FA"), the vowels ೠ, ಌ and ೡ with their signs, candrabindu, jihvamuliya and upadhmaniya are handled in segmentation and transliteration. Nakaara pollu (ೝ) is segmented as a syllable and passes through transliteration unchanged. The archaic vowel signs proposed to Unicode in 2014 are not encoded yet, so no software can support them.

## Prior art

See [docs/prior-art.md](docs/prior-art.md).

## Architecture

The crate is split into small modules, each with 1 job:

| Module | Job |
|---|---|
| `chars` | Classifies characters in the Kannada block |
| `akshara` | Splits text into aksharas |
| `normalize` | Unicode NFC plus Kannada fixes; Old Kannada modernization |
| `numerals` | Kannada digits: conversion, formatting, parsing |
| `translit` | 1 engine that romanizes and parses, driven by a table per scheme |
| `legacy` | Converts Nudi and Baraha glyph sequences to Unicode syllables and back, driven by the glyph table in `data/nudi.tsv` |

Scheme spellings and legacy glyph layouts are data tables read by 1 engine each. [AGENTS.md](AGENTS.md) has the full guide for contributors.

## Development

```sh
cargo test
PROPTEST_CASES=50000 cargo test --release --test props
cargo clippy --all-targets
web/build.sh && node web/test/smoke.mjs && node --test web/test/units.mjs
```

The minimum supported Rust version is 1.75.

## License

MIT. See [LICENSE](LICENSE).

If kannadi helps your project, a mention or a link to [prasad.tech](https://prasad.tech) is appreciated.
