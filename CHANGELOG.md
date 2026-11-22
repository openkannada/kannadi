# Changelog

## Unreleased

### Added

- The Kannada text normalization profile, [NORMALIZATION.md](NORMALIZATION.md): 3 forms (canonical, legacy, search) and 14 rules, each with counts from Kannada Wikipedia and Wikisource, and HarfBuzz rendering checks for the joiner rules. 75 test vectors in `normalization/vectors.tsv` and `vectors.json`.
- `normalize::normalize_as`, `normalize_counted`, `is_normalized`, `Form`, `Rule` and `Counts`.
- `kannadi normalize`: normalize text files or standard input to a form, with `--stats` (changes per rule) and `--check` (exit 1 if not normalized).
- Tests that clean converter output is canonical, on every fixture line and on random glyph sequences.
- `kannadi to-unicode` reads Word 97 to 2003 `.doc` files and `.rtf` files, and folders pick them up. Their text goes to `NAME.doc.txt` and `NAME.rtf.txt`, with runs in legacy fonts converted in their layout and other runs kept, and report rows give the output line. The `.doc` reader follows [MS-DOC] (piece table, character and paragraph properties, stylesheet, font table) and reads the compound file with the `cfb` crate; the RTF reader handles the font table, `\f`, `\'hh`, `\u` with `\uc`, `\upr`, fields and destinations. Footnotes, headers and footers, comments, endnotes and text boxes follow the body. Golden tests use files LibreOffice saved from a DOCX of fixture lines, property tests check that legacy runs convert and Unicode runs survive in both formats, and `crates/kannadi-cli/fuzz` has `cargo fuzz` targets for both readers.
- `legacy::windows_1252_char`: the character of 1 Windows-1252 byte.
- `kannadi-typing`, a new crate: ranked Kannada words for loosely typed English letters, from a 4-gram model over (letters, Kannada unit) pairs trained on the Aksharantar pairs, a beam search through a lexicon of 232,003 Kannada Wikipedia words, completions, strict Baraha and ITRANS readings, the previous word, and the user's picks. 89.19% top-1 and 96.77% top-5 on the Dakshina Kannada test set; IndicXlit published 77.18% top-1. The first word is what the letters spell: the strict reading for 1 or 2 letters ("ka" gives ಕ), and the best-fitting reading when the letters likely start a longer word ("namas" gives ನಮಸ್, then ನಮಸ್ಕಾರ); the strict reading is always in the first 5. The model (`model/kn.typing`) is under CC BY-SA 4.0.
- `kannadi-typing-wasm`, WebAssembly bindings for the typing engine, with a smoke test that times every keystroke.
- `kannadi type`: the ranked Kannada words for words typed in English letters, with `--prev`, `-n` and `--scores`.
- `kannadi-search`, a new crate: search tokens for Kannada text. Legacy font lines are converted, words split at Unicode word boundaries without breaking an akshara, spelling variants fold to 1 key (the profile's search form, a nasal with virama for ಂ, digits, case), and noun endings are removed with kannada-morph's paradigms, guarded by Wikipedia word counts. A Tantivy `TextAnalyzer` behind the `tantivy` feature, and a Postgres `unaccent` rules file with an example setup. On 1,000 noun queries over held-out Kannada Wikipedia sentences: 98.5% recall and 98.6% precision, against 47.8% and 99.1% for charabia 0.10.0 and 46.8% and 100% for ICU; on Karnataka government PDF text in legacy fonts, 95.9% and 97.4%, where the others find nothing.

### Changed

- Breaking: `normalize` now gives the profile's canonical form, so it changes text the 0.1.0 version left alone. Besides NFC, look-alike vowel letters and repeated marks, it builds ಔ from ಒ + ೌ, moves a nukta to its consonant, and settles joiners: ZWJ after a virama becomes ZWNJ, ರ + virama + ZWJ becomes ರ + ZWJ + virama, a joiner between a consonant and its virama goes, and joiners that change nothing are removed.

## 0.1.0 - 2026-11-18

### Added

Library (`kannadi`):

- Akshara segmentation (`aksharas`, `split`, `count_aksharas`), including conjuncts, nukta, ZWJ and ZWNJ after a virama, and Old Kannada letters. `Akshara::is_well_formed` finds stray signs, signs on vowel letters and doubled vowel signs.
- Normalization (`normalize`): NFC, look-alike vowel letters, repeated marks. `fix_zero_as_anusvara` for converted and scanned text.
- Modernization (`modernize`): ೞ to ಳ, ಱ to ರ, ೝ to ನ್.
- Transliteration between Kannada and ISO 15919, IAST and Baraha phonetic, lossless in both directions for Kannada text.
- Kannada numerals: digit conversion, formatting and parsing.
- Nudi and Baraha ANSI to Unicode (`legacy::nudi_to_unicode`), with a glyph table derived from the font files and reports of unknown glyphs and malformed syllables.
- Unicode to Nudi and Baraha ANSI (`legacy::unicode_to_nudi`), writing each akshara in the order typists stored it, with reports of unencodable characters.
- Akruti to Unicode and back (`legacy::to_unicode` and `legacy::from_unicode` with `Layout::Akruti`), with a glyph table derived from the fonts embedded in Karnataka Acts and checked on 908,075 glyphs from 35 documents. `legacy::Layout`, `UnknownLayout` and `syllable_starts`.
- Shree-Lipi to Unicode and back (`Layout::ShreeLipi`), with a glyph table derived from the Shree-Kan-0850 font the Udayavani newspaper served with its web pages, checked on 233,346 glyphs from 173 pages. `legacy::font_layout` gives Shree-Kan-0850 this layout; other Shree-Kan fonts stay legacy fonts with no table.
- Nudi B-Akshara to Unicode and back (`Layout::NudiB`), with a glyph table derived from the fonts embedded in Karnataka Legislative Assembly member pages, whose PDFs dropped the font's name, and in budget papers that name it NudiB-Akshar. `legacy::font_layout` gives NudiB-Akshar this layout; earlier it was read as Nudi. `Layout::draws_latin` for layouts whose fonts keep Latin letters at the ASCII codes.
- Kan-Eng to Unicode and back (`Layout::KanEng`), with a glyph table derived from the Kan-EngPraja and Kan-EngDeepa fonts embedded in Karnataka budget papers of 2002 to 2014, checked on 21,455 glyphs from 45 papers. `legacy::font_layout` gives every font named Kan-Eng this layout. The encoder puts the mahaprana mark on a letter when the layout draws the plain consonant only as a letter.
- `legacy::best_layout` breaks ties by fewer Latin letters, then fewer viramas.
- The encoder writes a consonant whose subscript the layout cannot draw as a half form with a visible virama.
- `chars::Class::is_mark`.
- Sound-alike keys (`phonetic::key`, `keys`, `latin_key`) at 3 levels, for fuzzy search and for suggesting Kannada words from loosely typed Latin letters: "namaskara" and ನಮಸ್ಕಾರ share a medium key. `kannadi-wasm` exports `phoneticKey` and `latinKey`, so the editor page can use them.
- Typo suggestions (`typos::find`): ರೃ at the start of a syllable suggests ಋ, as in ನೈರೃತ್ಯ for ನೈಋತ್ಯ. Sanskrit spells r + ṛ this way, so these are suggestions only. `kannadi-wasm` exports `typos`, with UTF-16 offsets, for editors that mark them.
- Typo rule `ra-virama-vocalic-r`: ರ್ followed by ಋ suggests ಋ, as in ನೈರ್ಋತ್ಯ for ನೈಋತ್ಯ. Nudi text with an arkavattu typed after ಋ converts to ರ್ಋ. ರ್ before other vowel letters is left alone.

Command-line tool (`kannadi-cli`):

- `kannadi to-unicode` and `kannadi to-nudi` for files and folders, with a per-file report of problems and `--strict`.
- `to-unicode` reports likely typos in the converted text as suggestions. They do not make `--strict` fail.
- Release builds with dist: archives for macOS (Apple silicon and Intel), Linux (x86_64 and arm64, glibc and musl) and Windows, with SHA-256 checksums, shell and PowerShell install scripts, and a Homebrew formula that installs the man pages and completions. Both crates have the metadata crates.io needs; the command-line tool installs with `cargo install kannadi-cli`.
- `kannadi completions SHELL` prints a completion script for bash, zsh, fish, PowerShell or Elvish, and `kannadi manpages DIR` writes man pages. Copies are kept in `crates/kannadi-cli/assets`, and a test fails when they fall behind the command line.
- Help lists the exit statuses and gives examples, and each command's help covers only its own options and default font. `--layout` takes `baraha` as another name for `nudi`, in any case, and completion scripts offer the layout names.
- Plain text, including UTF-16 and Windows-1252 input.
- HTML: converts only text set in a legacy font, found from `style`, `<style>` rules and `<font face>`.
- DOCX: converts only runs in a legacy font, following run, character and paragraph styles.
- PDF: extracts text and converts text in legacy fonts, including fonts renumbered behind a ToUnicode map.
- PDF: Ghostscript's glyph names `c<n>` are read as code n; strings drawn several times a fraction of a point apart, to look bold, are read once; short text in a font with a hidden name is not taken as a layout that draws Latin.
- PDF: `T*`, `'` and `"` move down by the text leading, as `TL` and `TD` set it, so the parts of a line that Acrobat Distiller draws with a leading of 0 stay on 1 line.
- PDF: the CID copy Acrobat makes of a WinAnsi font, for glyphs at codes Windows-1252 leaves undefined and at AD and B5, is read back to those codes, so its glyphs join the words around them. Named legacy CID fonts have their ToUnicode map read.
- Akruti in every format: font names give each run's layout in HTML, DOCX and PDF; `to-unicode --layout akruti` for text files; `--legacy-font "NAME=akruti"`. In PDFs, glyph names `G<n>` in renumbered lists are read as code n, fonts with hidden names are judged in each layout, and a gap that would break a syllable adds no space.

Editor page and bindings:

- `kannadi-wasm`: WebAssembly bindings for conversion in both directions, transliteration, normalization, segmentation, syllable checks and fixes, with offsets in UTF-16 code units.
- The editor page reads Akruti: DOCX and HTML text by its font, pasted and plain text in the layout that converts best (`legacy::best_layout`, which breaks ties by fewer viramas), chosen once per paste, file or selection. `kannadi-wasm`: `toUnicode` with a layout name, and `bestLayout`.
- A static Kannada editor in `web/`: open TXT, DOCX and HTML, converting only text set in Nudi or Baraha fonts; inline marks for unknown glyphs and malformed syllables with checked fixes; Baraha and ISO 15919 typing; normalize and modernize; akshara and word counts; save as DOCX, TXT, HTML or Nudi text; autosave in the browser. Self-hosted Noto Sans Kannada and Noto Serif Kannada.
- Landing pages for Nudi to Unicode, Unicode to Nudi, Baraha to Unicode, Kannada typing and English to Kannada typing, built from 1 template with JSON-LD, Open Graph cards, `llms.txt`, and a sitemap when the site URL is set.

Data and tests:

- A benchmark corpus of 207 lines from 31 Karnataka government pages of 2002, with a data card.
- Golden fixtures from Karnataka government pages and Acts, property tests, and a differential test against vidyut-lipi.

Font tools:

- `tools/fontqa`: HarfBuzz shaping tests for Kannada fonts, results for 41 fonts (146 faces) with a data card, proof sheets, and fixes for broken `knd2` tables, broken `morx` tables and missing ೄ, ೢ and ೣ. `measure` and `gate` commands, and a mean stroke width gate.
- `tools/glyphdraft`: drafts U+0CDC, U+0CDD and U+0CF3 into a FontForge source, matched to the font's stems, weight, heights and spacing.
- fontqa `export`: the test strings as FontBakery shaping tests (forbidden glyphs, rare characters apart, collisions) and Shaperglot `kn_Knda` checks (conjuncts, arkavattu), with draft upstream texts. fontqa `dashboard`: a static coverage page built from the results.

Project:

- `CITATION.cff`, `AGENTS.md` and `CONTRIBUTING.md`.
- Licenses: code and glyph tables under MIT OR Apache-2.0 (`LICENSE-MIT`, `LICENSE-APACHE`), docs under CC BY 4.0, copyright "The Open Kannada Authors". `REUSE.toml` gives the license of every file, `NOTICE.md` lists third-party material, and `TRADEMARKS.md` says how the names and logo may be used.
- Contributions are signed off under the Developer Certificate of Origin. CI checks the `Signed-off-by` line of each commit in a pull request, and runs `reuse lint`.

### Changed

- The Shree-Lipi fixture holds 140 lines written for kannadi and encoded with its own table, with 22 words retyped in the typing habits of the newspaper pages. It replaces 130 lines of Udayavani pages and uses every code and table entry they used.

### Fixed

- `to-unicode` reads Unicode PDFs whole (#42). Conjuncts and vowel signs were lost from PDFs Chrome makes: fonts that are not legacy are now read through their own ToUnicode map, which may give several characters per glyph, and `/ActualText` spans replace the glyphs drawn in them. Glyphs with no Unicode text come out as U+FFFD and are reported as not converted.
- Unicode Kannada in legacy text is no longer reported as unknown glyphs (#43). Kannada characters, the dandas, ZWNJ and ZWJ pass through `legacy::to_unicode` as they are, and `to-unicode` keeps the lines of a text file that are already Unicode, which `legacy::is_unicode` tells by comparing Kannada characters with the layout's glyph codes, converts the other lines, and still suggests likely typos in kept lines. `kannadi-wasm` exports it as `isUnicode(text, layout)`.
- fontqa outline flattening: a TrueType quadratic with several off-curve points is split at its implied on-curve points, and a closed contour with no on-curve point is read. Stroke measurements on TrueType fonts were slightly off before.
- fontqa manifest: Navilu is OFL-1.1 with Reserved Font Name Navilu, not GPL.
- glyphdraft: side bearings come from the median of the font's consonants, not from KA, whose bearings can be an outlier.
