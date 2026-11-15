# Legacy converter design: Nudi and Baraha to Unicode, and back

Status: design. No code yet. The test corpus comes first; see [corpus-plan.md](corpus-plan.md).

## The problem

Before Unicode Kannada was usable on Windows, Kannada text was typed with Nudi (from the Kannada Ganaka Parishat, adopted by the Government of Karnataka) and Baraha. Their older versions stored **glyph codes**, not characters. A file says `«eÁÕ£À`; it shows as ವಿಜ್ಞಾನ only because the paragraph is set in a Nudi font whose glyphs sit at those Latin-1 code points.

Government orders, court records, and newspaper and publisher archives from those years are still in this form. Without conversion they cannot be searched, indexed, read aloud, or used by language models. Search data from November 2026 shows about 27,100 searches a month in India for "nudi to unicode".

## Why it is harder than a lookup table

Each issue below is known from the bug reports in ascii2unicode's public ChangeLog and from how glyph-encoded Indic fonts work in general. Each one becomes a test fixture.

1. **Storage order is visual order.** A subscript consonant (ottu, vattakshara) is stored after the syllable it hangs under, including after its vowel sign. ತಿಮ್ಮಿ is stored as ತಿ, ಮಿ, then the ಮ-ottu glyph. In Unicode it is ತ ಿ ಮ ್ ಮ ಿ: the ottu becomes virama plus consonant, and the vowel sign moves to the end of the cluster.
2. **Arkavattu (repha).** The ರ್ that sits at the right of a cluster, as in ಅರ್ಕ, is a separate glyph stored after the consonant. In Unicode ರ ್ comes first. A known failure: ಅರ್ಕ converted as ಅಕ್ರ.
3. **Vowel signs drawn in parts.** ೀ, ೇ, ೈ, ೊ and ೋ are often built from 2 glyphs, such as a base sign plus a length mark. Some combinations are canonical Unicode pairs (ೆ + ೕ is ೇ), others are not (a ು-shaped hook after ೆ means ೊ, not ು).
4. **Split and special consonant shapes.** Some consonants are drawn from 2 glyphs, and some consonant plus vowel pairs (ಪು, ಫು, ವು) have their own glyph. Known failures: ಮೊ read as ವೊ, ಪು read as ಪ.
5. **Special conjuncts.** ಜ್ಞ, ಕ್ಷ and ಶ್ರೀ can be single glyphs. Known failure: ಜ್ಞ in ವಿಜ್ಞಾನ.
6. **Windows-1252 surprises.** Glyphs at bytes 0x80 to 0x9F come out of Word and browsers as the Unicode characters Windows-1252 assigns to them, such as ‚ ƒ „ … ’ “ ” • ™ Š and Œ. The decoder must accept both the raw byte and its Windows-1252 Unicode meaning.
7. **Mixed text.** A DOCX has Nudi runs and English runs side by side. Only text set in a legacy font may be converted.
8. **Digits.** Legacy fonts may draw ASCII digit codes as Kannada digits. The user decides which they want.

## Architecture

```text
input file ──► extract runs (with font name) ──► legacy runs ──► decode ──► glyph tokens
                                               └► other runs pass through unchanged
glyph tokens ──► group into visual syllables ──► reorder to logical order ──► compose signs
            ──► kannadi::normalize ──► validate with kannadi::aksharas ──► Unicode text + report
```

### 1. Font tables as data

Each legacy font family is one table file (`fonts/nudi.toml`, `fonts/baraha.toml`), not code:

- the font names it covers, as they appear in DOCX `w:rFonts` and HTML `font-family`;
- for each code point: the glyph's role (base consonant, vowel, vowel sign part, ottu of consonant X, arkavattu, modifier, digit, punctuation) and its Unicode value;
- multi-glyph sequences that form one letter (split consonants, special conjuncts).

Tables are written from the fonts themselves (each glyph rendered and identified by a Kannada reader) and checked against the corpus. ascii2unicode uses one table for both Nudi and Baraha, which suggests the two share most of a layout; this is verified per font, not assumed.

### 2. Decode and tokenise

Map each input character to its glyph code (handling the Windows-1252 cases), then match the longest glyph sequence in the table. Unknown codes become `Unknown(code)` tokens and are reported, never dropped.

### 3. Group and reorder

Walk the tokens and build one visual syllable at a time: a base (consonant or vowel), then any vowel sign parts, ottus, arkavattu and modifiers that follow it, until the next base. Emit each syllable in Unicode logical order:

```text
[ರ ್ if arkavattu] base [nukta] (್ ottu-consonant)* vowel-sign modifiers
```

This is a small state machine with unit tests for each rule, in the same style as `akshara.rs`.

### 4. Compose, normalise, validate

Join vowel sign parts into one sign, then run `kannadi::normalize`, which already handles NFC pairs and the digit-zero versus anusvara mix-up. Finally split the result with `kannadi::aksharas` and flag any segment that is not a well-formed akshara (a stray sign, a virama with no consonant). A clean document has none.

### 5. Report

Every conversion returns the text plus a report: characters converted, unknown glyph codes with their positions, and malformed aksharas. The command-line tool prints it and, with `--strict`, exits non-zero if anything is unknown.

### Reverse: Unicode to legacy

Offices that still print with legacy fonts need this. Split the Unicode text into aksharas, choose glyphs for each from the font table, and write them in storage order (vowel sign before ottu, arkavattu last). Where a font has 2 ways to draw something, the table marks one as preferred. The property that must hold: for normalised, well-formed Unicode text, converting to legacy and back gives the same text. The other direction (legacy to Unicode to legacy) is not always the identity, because one Unicode string can be drawn with different glyph sequences.

## File formats

| Format | How legacy text is found | Notes |
|---|---|---|
| TXT | User names the font family (`--from nudi`) | Optional detection: score the share of characters in the glyph ranges that do not form English words. |
| HTML | `font-family` in `style` attributes and stylesheets, and `<font face>` | Rewrites the font declaration to a Unicode Kannada font. |
| DOCX | Run properties `w:rFonts` | Uses a zip and XML reader; only legacy-font runs change, styles and layout are kept. |

## Public API sketch

```rust
pub enum LegacyFont { Nudi, Baraha }

pub struct Conversion {
    pub text: String,
    pub unknown: Vec<(usize, char)>,
    pub malformed: Vec<std::ops::Range<usize>>,
}

pub fn legacy_to_unicode(text: &str, font: LegacyFont) -> Conversion;
pub fn unicode_to_legacy(text: &str, font: LegacyFont) -> Result<String, Unmappable>;
```

This goes in the core crate behind a `legacy` feature, so the browser build can include or leave out the tables.

## Testing

- **Golden corpus:** every document in the corpus converts to its verified Unicode text with zero differences. See [corpus-plan.md](corpus-plan.md).
- **Per-rule unit tests:** one test for each issue listed above, built from the smallest example that shows it.
- **Property test:** for random well-formed Unicode Kannada, `legacy_to_unicode(unicode_to_legacy(x)) == x` for each font.
- **Ablation:** each reordering or composition rule is removed in turn to confirm a test fails.
- **Metric:** akshara error rate (edit distance over `kannadi::aksharas`) on real documents, reported per document in CI.

## Licensing

ascii2unicode is GPL 3.0 or later. Its code and tables are not copied. If its author agrees to license the mapping table and test files under MIT or Apache 2.0, they can be used with credit, which would save time on step 1. A message proposing this has been drafted.

## Open questions

1. Which Nudi and Baraha font versions appear in real documents, and how far their layouts differ. The corpus answers this.
2. Whether old Nudi DOC files (Word 97 binary) are common enough to support directly, or whether users should save as DOCX first.
3. Whether to ship detection for TXT in version 1 or require `--from`.
