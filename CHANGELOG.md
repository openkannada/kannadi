# Changelog

## Unreleased

### Added

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
- `Akshara::is_well_formed`: finds stray signs, signs on vowel letters and doubled vowel signs in Unicode text.
- `kannadi-wasm`: `malformed`, `fixes`, `countAksharas`, `modernize`, and unknown glyph offsets in the converted text.
- Nudi and Baraha ANSI to Unicode (`legacy::nudi_to_unicode`), with a glyph table derived from the font files, reports of unknown glyphs and malformed syllables, and fixtures from Karnataka government pages.
