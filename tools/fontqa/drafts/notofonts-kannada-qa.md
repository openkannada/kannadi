Draft pull request for notofonts/kannada. Not submitted.

Title: Add forbidden-glyph and collision tests for Kannada

`qa/shaping_tests/kannada.json` holds 341 regression tests with expected glyph names. This adds test files that need no expectations, so they keep working when glyph names change:

- `kannada-fontqa-forbidden.json`: 2,135 strings (every consonant with every vowel sign, every 2-consonant cluster, the arkavattu, clusters with signs). None may shape to `.notdef` or a dotted circle. Noto Sans Kannada and Noto Serif Kannada pass it today.
- `kannada-fontqa-collisions.json`: 2,119 strings with marks, run through collidoscope (marks only, 3% area). Both fonts pass it today.
- `kannada-fontqa-forbidden-rare.json`: 11 strings with rare characters. Both fonts fail it on U+0CDC until that glyph is added, so it could land with that change.

FontBakery 1.1.0 reads the test folder from a `shaping:` key. The current `fontbakery.yml` uses the older `com.google.fonts/check/shaping:` key, which 1.1.0 skips, so this PR renames it:

    shaping:
        test_directory: qa/shaping_tests

The existing `kannada.json` regression tests run under the new key too.

The strings come from the kannadi fontqa suite, which also reports conjunct and arkavattu failures that these 2 checks do not cover.
