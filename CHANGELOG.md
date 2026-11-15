# Changelog

## Unreleased

### Added

- Akshara segmentation (`aksharas`, `split`, `count_aksharas`), including conjuncts, nukta, ZWJ and ZWNJ after a virama, and Old Kannada letters.
- Normalisation (`normalize`): NFC, look-alike vowel letters, repeated marks. `fix_zero_as_anusvara` for converted and scanned text.
- Modernisation (`modernize`): ೞ to ಳ, ಱ to ರ, ೝ to ನ್.
- Transliteration between Kannada and ISO 15919, IAST and Baraha phonetic, lossless in both directions for Kannada text.
- Kannada numerals: digit conversion, formatting and parsing.
- Golden fixtures, property tests, and a differential test against vidyut-lipi.
- Design for the Nudi and Baraha legacy converter, and the test corpus plan.
- MIT licence and `CITATION.cff`.
