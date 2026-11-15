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
- Nudi and Baraha ANSI to Unicode (`legacy::nudi_to_unicode`), with a glyph table derived from the font files, reports of unknown glyphs and malformed syllables, and fixtures from Karnataka government pages.
- `tools/fontqa`: HarfBuzz shaping tests for Kannada fonts, results for 41 open fonts (146 faces) with a data card, proof sheets, and fixes for broken `knd2` tables, broken `morx` tables and missing ೄ, ೢ and ೣ.
