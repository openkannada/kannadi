Draft pull request for googlefonts/shaperglot. Not submitted.

Title: Add Kannada conjunct and arkavattu checks

Shaperglot checks that a font covers Kannada's characters, but not that it forms conjuncts. A font can have every glyph and still show an explicit virama in every cluster: Gubbi does, under new-order shaping, in HarfBuzz.

This adds 2 `kn_Knda` checks to `manual_checks.toml`, both `ShapingDiffers`:

- **Consonant clusters form conjuncts**: 1,471 clusters, each shaped twice, with no features changed and with `-akhn -rphf -blwf -half -pstf -cjct`. A font that forms the conjunct gives 2 different outputs.
- **Ra before a consonant forms the arkavattu**: 56 strings, each shaped with and without `rphf`. ರ್ರ is left out: many fonts give it a subscript ra instead.

Pairs with ZWNJ or ZWJ would not work: `ShapingDiffers` compares the whole output, and a joiner always adds a glyph. `ignore_notdefs = true` keeps fonts without the historic letters ಱ and ೞ from failing on 2 identical `.notdef` outputs.

Measured with shaperglot-cli 1.2.2 built with these checks appended to `manual_checks.toml`:

- Noto Sans Kannada and Noto Serif Kannada pass both (100%).
- Gubbi 1.3 fails the conjunct check on 1,396 clusters, the same count as the kannadi fontqa suite.
- Gubbi with its `knd2` subscript lookups registered passes both.
