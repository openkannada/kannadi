# Changelog

## Unreleased

### Added

- `kannadi` reads Akruti: font names give each run's layout in HTML, DOCX and PDF; `to-unicode --layout akruti` for text files; `--legacy-font "NAME=akruti"`. In PDFs, glyph names `G<n>` from Acrobat Distiller are read as code n, fonts with hidden names are judged in each layout, and a gap before a sign or subscript adds no space.
- Akruti to Unicode and back (`legacy::to_unicode` and `legacy::from_unicode` with `Layout::Akruti`), with a glyph table derived from the fonts embedded in Karnataka Acts and checked on 908,075 glyphs from 35 documents, and a fixture of 125 lines read against the pages. `legacy::Layout` and `UnknownLayout`, `syllable_starts`; `nudi_to_unicode` and `unicode_to_nudi` stay as the Nudi calls.
- `tools/glyphdraft`: drafts U+0CDC, U+0CDD and U+0CF3 into a FontForge source, matched to the font's stems, weight, heights and spacing. fontqa `measure` and `gate` commands, and a mean stroke width gate.
- Akshara segmentation (`aksharas`, `split`, `count_aksharas`), including conjuncts, nukta, ZWJ and ZWNJ after a virama, and Old Kannada letters.
- Normalization (`normalize`): NFC, look-alike vowel letters, repeated marks. `fix_zero_as_anusvara` for converted and scanned text.
- Modernization (`modernize`): ೞ to ಳ, ಱ to ರ, ೝ to ನ್.
- Transliteration between Kannada and ISO 15919, IAST and Baraha phonetic, lossless in both directions for Kannada text.
- Kannada numerals: digit conversion, formatting and parsing.
- Golden fixtures, property tests, and a differential test against vidyut-lipi.
- Design for the Nudi and Baraha legacy converter, and the test corpus plan.
- `chars::Class::is_mark`.
- MIT license and `CITATION.cff`.
- `kannadi-wasm`: WebAssembly bindings for `legacy::nudi_to_unicode`, `transliterate`, `normalize` and `aksharas`, with report offsets in UTF-16 code units.
- A static converter page in `web/`: paste text or open a TXT file, convert Nudi or Baraha to Unicode in the browser, see unknown glyphs and malformed syllables, then copy or download the result. CI builds it and checks the package in Node.
- The page becomes a Kannada editor: open TXT, DOCX and HTML, converting only text set in Nudi or Baraha fonts; inline marks for unknown glyphs and malformed syllables with checked fixes; Baraha and ISO 15919 typing; normalize and modernize; akshara and word counts; save as DOCX, TXT or HTML; autosave in the browser. Self-hosted Noto Sans Kannada and Noto Serif Kannada.
- Landing pages for Nudi to Unicode, Unicode to Nudi, Baraha to Unicode, Kannada typing and English to Kannada typing, built from 1 template with SoftwareApplication and FAQPage JSON-LD, Open Graph cards, `llms.txt`, and a sitemap and canonical links when the site URL is set. Typing pages open in Baraha mode with a letter chart computed by the transliterator.
- The editor saves and copies text as Nudi glyph codes. `kannadi-wasm`: `unicodeToNudi` and `encodeWindows1252`.
- `Akshara::is_well_formed`: finds stray signs, signs on vowel letters and doubled vowel signs in Unicode text.
- `kannadi-wasm`: `malformed`, `fixes`, `countAksharas`, `modernize`, and unknown glyph offsets in the converted text.
- Nudi and Baraha ANSI to Unicode (`legacy::nudi_to_unicode`), with a glyph table derived from the font files, reports of unknown glyphs and malformed syllables, and fixtures from Karnataka government pages.
- `tools/fontqa`: HarfBuzz shaping tests for Kannada fonts, results for 41 fonts (146 faces) with a data card, proof sheets, and fixes for broken `knd2` tables, broken `morx` tables and missing ೄ, ೢ and ೣ.

### Fixed

- fontqa outline flattening: a TrueType quadratic with several off-curve points is split at its implied on-curve points, and a closed contour with no on-curve point is read. Stroke measurements on TrueType fonts were slightly off before.
- fontqa manifest: Navilu is OFL-1.1 with Reserved Font Name Navilu, not GPL.
