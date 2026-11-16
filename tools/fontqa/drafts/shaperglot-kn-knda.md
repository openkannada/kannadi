Draft pull request for googlefonts/shaperglot. Not submitted.

Title: Add Kannada conjunct and arkavattu checks

Shaperglot checks that a font covers Kannada's characters, but not that it forms conjuncts. A font can have every glyph and still show an explicit virama in every cluster: Gubbi does, under new-order shaping, in HarfBuzz.

This adds 2 `kn_Knda` checks to `manual_checks.toml`, both `ShapingDiffers`:

- **Consonant clusters form conjuncts**: 1,471 clusters, each paired with the same text plus ZWNJ after the virama, which asks for an explicit virama.
- **Ra before a consonant forms the arkavattu**: 57 strings, each paired with ra + ZWJ + virama, which asks for ra without the reph.

The pairs come from the kannadi fontqa suite. In its run over 41 open Kannada fonts, the conjunct check separates Gubbi (1,396 failures before its fix) and NudiParijatha (all clusters) from the fonts that work.

Not verified: a Shaperglot build with these checks; the file parses as TOML and follows the `ShapingDiffers` fields in shaperglot-lib 1.2.2.
