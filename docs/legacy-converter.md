# Legacy converter design

Status: Nudi and Baraha to Unicode and Unicode to Nudi and Baraha are implemented in `crates/kannadi/src/legacy/`, with the table in `crates/kannadi/data/nudi.tsv`. Akruti uses the same engine with `crates/kannadi/data/akruti.tsv`; see [akruti.md](akruti.md). The sources for Shree-Lipi are in [shree-lipi.md](shree-lipi.md).

## The problem

Before Unicode Kannada worked well on Windows, Kannada text was typed with Nudi and Baraha. Their older versions stored glyph codes in place of characters. A file contains `«eÁÕ£À`, and it shows as ವಿಜ್ಞಾನ only because the paragraph is set in a Nudi font that draws Kannada shapes at those Latin-1 code points.

Government orders, court records, and newspaper and publisher archives from those years are still stored this way. Until they are converted, they cannot be searched, indexed, read aloud or used by language models.

## Why a lookup table is not enough

Each issue below becomes a test fixture.

1. **Storage order follows the drawing.** A subscript consonant (ottu) is stored after the syllable it sits under, including after its vowel sign. ತಿಮ್ಮಿ is stored as ತಿ, ಮಿ, then the ಮ ottu glyph. In Unicode it is ತ ಿ ಮ ್ ಮ ಿ. The ottu becomes virama plus consonant, and the vowel sign moves to the end of the cluster.
2. **Arkavattu.** The ರ್ drawn at the right of a cluster, as in ಅರ್ಕ, is a separate glyph stored after the consonant. In Unicode, ರ ್ comes first.
3. **Vowel signs drawn in parts.** ೀ, ೇ, ೈ, ೊ and ೋ are often built from 2 glyphs. Some pairs match Unicode's own pairs (ೆ + ೕ is ೇ). Others need a rule: a ು-shaped hook after ೆ means ೊ.
4. **Split and special shapes.** Some consonants are drawn from 2 glyphs, and some consonant and vowel pairs (ಪು, ಫು, ವು) have a glyph of their own.
5. **Special conjuncts.** ಜ್ಞ, ಕ್ಷ and ಶ್ರೀ can be single glyphs.
6. **Windows-1252.** Glyphs at bytes 0x80 to 0x9F come out of Word and browsers as the characters Windows-1252 assigns to them, such as ‚ „ … ’ “ ” and ™. The decoder accepts both forms.
7. **Mixed text.** A DOCX has legacy-font runs and English runs side by side. Only text set in a legacy font is converted.
8. **Digits.** The user chooses Kannada or ASCII digits in the output.

## Where the mapping tables come from

The tables are derived here, from public material:

1. **The font files.** Each legacy font maps byte values to glyphs through its `cmap` table. For every mapped code point, the glyph is rendered and identified by a Kannada reader. Glyph names in the `post` table, and any `GSUB` ligature rules, are used as hints where the font has them. Older 8-bit fonts usually have no `GSUB`, so rendering is the main source.
2. **Real documents.** Converted text is compared with the document as displayed in its own font. Every difference becomes a table fix and a test.
3. **Round trips.** For well-formed Unicode text, converting to legacy and back must give the same text.

No code or tables from other converters are copied.

## Architecture

```text
input file ─► extract runs with font names ─► legacy runs ─► decode ─► glyph tokens
                                             └► other runs pass through
glyph tokens ─► group into visual syllables ─► reorder to logical order ─► join vowel sign parts
            ─► kannadi::normalize ─► check with kannadi::aksharas ─► Unicode text and a report
```

### Font tables as data

Each font family is 1 data file, such as `fonts/nudi.toml`. It lists:

- the font names it covers, as they appear in DOCX `w:rFonts` and HTML `font-family`;
- for each code point, the glyph's role (consonant, vowel, vowel sign part, ottu of a consonant, arkavattu, modifier, digit, punctuation) and its Unicode value;
- glyph sequences that form 1 letter.

### Decode and tokenize

Each input character is mapped to its glyph code, then matched against the longest glyph sequence in the table. Unknown codes become `Unknown(code)` tokens and appear in the report. They are never dropped.

### Group and reorder

Tokens are grouped 1 visual syllable at a time: a base, then the vowel sign parts, ottus, arkavattu and modifiers after it, up to the next base. Each syllable is written in Unicode order:

```text
[ರ ್ if arkavattu] base [nukta] (್ ottu consonant)* vowel sign, modifiers
```

### Join, normalize, check

Vowel sign parts are joined into 1 sign. Then `kannadi::normalize` runs, which already fixes Unicode pairs and the digit zero typed for an anusvara. Finally `kannadi::aksharas` splits the result, and any segment that is not a well-formed akshara is flagged.

### Report

Every conversion returns the text with a report: characters converted, unknown glyph codes with positions, and malformed aksharas. The command-line tool prints it. With `--strict` it exits with an error if anything is unknown.

### Unicode to legacy

Offices that still print with legacy fonts need this direction. The text is split into aksharas, glyphs are chosen for each from the font table, and they are written in storage order:

```text
consonant shape [mahaprana] [head stroke] vowel sign parts, ottus, ೃ ೕ ೖ, arkavattu, ಂ ಃ
```

The consonant shape is a letter glyph that already carries the sign (ಕಿ, ಮೆ) when the table has one. For no sign, ಿ, ು and ೂ, the shape is the letter form without a head stroke (ಜ, ಲ, ಟ) when the table has one, or else the base plus the head stroke. Other signs replace the head stroke, so they go on the base form. Where a font can draw something 2 ways, a fourth column in the table says when to prefer each glyph. The order and preferences were counted from 9,047 words typed on 35 Karnataka government pages from 2002, archived by the Internet Archive under `kar.nic.in/rdpr/kan-www/` and `kar.nic.in/finance/bud2002-kan/`. On those words, 9,029 encode to the exact bytes that were typed.

## File formats

| Format | How legacy text is found | Notes |
|---|---|---|
| TXT | The whole file is read as legacy text in the layout the command names | Automatic detection is not done. |
| HTML | `font-family` in styles, and `<font face>` | The font declaration is changed to a Unicode Kannada font. |
| DOCX | Run properties `w:rFonts` | Only legacy-font runs change. Styles and layout stay. |

## Public API

```rust
pub fn to_unicode(text: &str, layout: Layout, options: &Options) -> Conversion; // text, unknown, malformed
pub fn from_unicode(text: &str, layout: Layout) -> Encoding;                    // text, unencodable
pub fn decode_windows_1252(bytes: &[u8]) -> String;
```

Nudi and Baraha ANSI share 1 layout, `Layout::Nudi`; `nudi_to_unicode` and `unicode_to_nudi` are the same calls with that layout. Each layout has its own table in `data/`, parsed once per direction. A new layout is a table and an entry in `Layout`.

## Testing

- **Corpus:** every document in the corpus converts to its verified Unicode text with no differences. See [corpus-plan.md](corpus-plan.md).
- **Rules:** 1 unit test for each issue listed above, built from the smallest example that shows it.
- **Round trip:** for random well-formed Unicode Kannada, `nudi_to_unicode(unicode_to_nudi(x)) == x`.
- **Ablation:** each reordering and joining rule is removed in turn to confirm a test fails.
- **Accuracy:** akshara error rate (edit distance over `kannadi::aksharas`) on real documents, reported per document.

## Open questions

1. Which other legacy layouts appear in real documents. So far, Nudi and Baraha ANSI fonts share 1 layout, and Akruti has its own.
2. Whether Word 97 DOC files are common enough to read directly, or whether users should save as DOCX first.
3. Whether TXT input needs automatic font detection in version 1.
