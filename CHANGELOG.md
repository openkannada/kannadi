# Changelog

## Unreleased

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
- `chars::Class::is_mark`.

Command-line tool (`kannadi-cli`):

- `kannadi to-unicode` and `kannadi to-nudi` for files and folders, with a per-file report of problems and `--strict`.
- Plain text, including UTF-16 and Windows-1252 input.
- HTML: converts only text set in a legacy font, found from `style`, `<style>` rules and `<font face>`.
- DOCX: converts only runs in a legacy font, following run, character and paragraph styles.
- PDF: extracts text and converts text in legacy fonts, including fonts renumbered behind a ToUnicode map.

Editor page and bindings:

- `kannadi-wasm`: WebAssembly bindings for conversion in both directions, transliteration, normalization, segmentation, syllable checks and fixes, with offsets in UTF-16 code units.
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

- MIT license, `CITATION.cff`, `AGENTS.md` and `CONTRIBUTING.md`.

### Fixed

- fontqa outline flattening: a TrueType quadratic with several off-curve points is split at its implied on-curve points, and a closed contour with no on-curve point is read. Stroke measurements on TrueType fonts were slightly off before.
- fontqa manifest: Navilu is OFL-1.1 with Reserved Font Name Navilu, not GPL.
- glyphdraft: side bearings come from the median of the font's consonants, not from KA, whose bearings can be an outlier.
