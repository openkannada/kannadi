Draft pull request for notofonts/kannada. Not submitted.

Title: Add forbidden-glyph and collision tests for Kannada

`qa/shaping_tests/kannada.json` holds 341 regression tests with expected glyph names. This adds 2 test files that need no expectations, so they keep working when glyph names change:

- `kannada-fontqa-forbidden.json`: 2,146 strings (every consonant with every vowel sign, every 2-consonant cluster, the arkavattu, clusters with signs, rare characters). None may shape to `.notdef` or a dotted circle.
- `kannada-fontqa-collisions.json`: the 2,119 of those strings that carry marks, run through collidoscope (marks only, 3% area).

Both run under the existing `shaping` configuration in `fontbakery.yml`. With FontBakery 1.1.0, the current Noto Sans Kannada and Noto Serif Kannada fail the forbidden-glyph test only on U+0CDC, which neither font has yet.

The strings come from the kannadi fontqa suite, which also reports conjunct and arkavattu failures that these 2 checks do not cover.
