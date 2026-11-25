# The kannadi library

The `kannadi` crate: legacy font conversion, akshara segmentation, normalization, transliteration and Old Kannada. The [README](../README.md) has the overview.

## Example

```rust
use kannadi::{aksharas, fix_zero_as_anusvara, modernize, normalize, numerals, transliterate, Scheme};

// Aksharas: the units a reader sees as 1 letter.
let parts: Vec<&str> = aksharas("ಸ್ತ್ರೀಯರು").map(|a| a.text).collect();
assert_eq!(parts, ["ಸ್ತ್ರೀ", "ಯ", "ರು"]);

// Normalization fixes text that looks right but compares wrong.
assert_eq!(normalize("ಕಿ\u{0CD5}ರ್ತಿ"), "ಕೀರ್ತಿ"); // vowel sign typed in 2 parts

// For converted or scanned text: a digit zero typed for an anusvara.
assert_eq!(fix_zero_as_anusvara("ಬೆ೦ಗಳೂರು"), "ಬೆಂಗಳೂರು");

// Old Kannada to modern spelling, for search.
assert_eq!(modernize("ಅಱಿವು ಬಾೞ್"), "ಅರಿವು ಬಾಳ್");

// Transliteration, both ways.
assert_eq!(transliterate("ಕನ್ನಡ ನಾಡು", Scheme::Kannada, Scheme::Iso15919), "kannaḍa nāḍu");
assert_eq!(transliterate("kannaDa nADu", Scheme::Baraha, Scheme::Kannada), "ಕನ್ನಡ ನಾಡು");

// Numerals.
assert_eq!(numerals::format(1956), "೧೯೫೬");
```

Run `cargo run --example demo` to see each feature on 1 sentence.

## Legacy fonts

Nudi (up to version 5) and Baraha's ANSI fonts store glyph codes in place of letters, so the text reads as `PÀ£ÀßqÀ` outside the font. `legacy::nudi_to_unicode` converts it:

```rust
use kannadi::legacy::{nudi_to_unicode, Options};

let converted = nudi_to_unicode("PÀ£ÀßqÀ", &Options::default());
assert_eq!(converted.text, "ಕನ್ನಡ");
assert!(converted.unknown.is_empty());
```

- **How the table was built.** The glyph table in [nudi.tsv](../crates/kannadi/data/nudi.tsv) comes from the Nudi font files. Each glyph was rendered and matched against the Unicode version of the same typeface, which HarfBuzz labels. The table was then checked against typed letter charts and Karnataka government pages from 2002.
- **Nothing is dropped silently.** Glyphs the converter cannot place are kept in the output and listed in `unknown`, with their byte offsets. Syllables that cannot be valid Kannada are listed in `malformed`: a mark with no letter, 2 vowel signs on 1 consonant, a sign on a vowel letter, or a consonant shape typed without its sign. Spacing glyphs carry no text and are the only glyphs dropped.
- **Unicode already in the text.** Kannada characters, the dandas, ZWNJ and ZWJ are never legacy glyph codes, so they pass through unchanged and are not reported as unknown glyphs. `legacy::is_unicode(text, layout)` is true for text with more Kannada characters than characters above ASCII the layout uses as glyph codes, such as À in Nudi: text that is already Unicode and needs no conversion, even with symbols such as 30°C, © or curly quotes.
- **Digits.** The `k` variants of the Nudi fonts draw Kannada digits at the ASCII digit positions. For text in those fonts, set `Options::kannada_digits`.
- **Typed zeros.** A zero typed between 2 Kannada letters in place of an anusvara (ಪ್ರಾರ0ಭ) becomes ಂ by default.
- **Limits.**
  - Some glyphs look alike: the ೂ in ವೊ and ಮೊ, the ು in ವು and the tail of ಮ, and the subscript ಜ and ಞ. A document that used the wrong key converts to what was typed.
  - English text set in the same font is converted too, so pass only the legacy-font runs.

Run `cargo run --example nudi -- file.txt` to convert a Windows-1252 text file.

Other layouts go through `legacy::to_unicode` and `legacy::from_unicode` with a `Layout`. `Layout::Akruti` covers AkrutiKndPadma and its weights, used in Karnataka Acts of 2001 to 2003, `Layout::ShreeLipi` covers Shree-Kan-0850, used by the Udayavani newspaper's web pages, `Layout::NudiB` covers Nudi B-Akshara, the bilingual Nudi font of Karnataka budget and Assembly papers, and `Layout::KanEng` covers the bilingual Kan-Eng fonts, such as Kan-EngPraja and Kan-EngDeepa, of Karnataka budget papers of 2002 to 2014:

```rust
use kannadi::legacy::{to_unicode, Layout, Options};

let converted = to_unicode("dâ}â", Layout::Akruti, &Options::default());
assert_eq!(converted.text, "ಕನ");
```

The Akruti table was derived from the fonts embedded in public PDFs and checked against the pages; [docs/akruti.md](akruti.md) has the method and results. The Shree-Lipi table was derived from the font Udayavani served with its pages and checked against them; [docs/shree-lipi.md](shree-lipi.md#the-shree-lipi-table) has the method and results. The Nudi B-Akshara table was derived from the fonts embedded in budget and Assembly PDFs; [docs/nudi-b.md](nudi-b.md) has the method and results. The Kan-Eng table was derived from the fonts embedded in budget PDFs; [docs/kan-eng.md](kan-eng.md) has the method and results. The command-line tool reads these layouts too. So does the editor page: by font name in DOCX and HTML files, and otherwise (pasted text, text files, HTML with no font, the Convert button) in the layout that converts best, chosen by `legacy::best_layout` once for all such text in a paste, a file or a selection.

`legacy::unicode_to_nudi` goes the other way, for documents that must still be printed in a legacy font:

```rust
use kannadi::legacy::unicode_to_nudi;

let encoded = unicode_to_nudi("ಕನ್ನಡ");
assert_eq!(encoded.text, "PÀ£ÀßqÀ");
assert!(encoded.is_clean());
```

- **Storage order.** Each akshara is written the way typists stored it: the consonant shape, the vowel sign, the subscripts, ೃ and the length marks ೕ and ೖ, the arkavattu, then ಂ or ಃ. Where the layout has 2 glyphs for 1 sign, such as the ು of ವು or a second subscript ರ, the table marks which one the font uses there. On 9,047 words from 35 Karnataka government pages from 2002, 9,029 encode to the exact bytes that were typed. The rest differ in typing habits: a zero for ಂ, an extra spacing glyph, or ರ್ typed as its own syllable.
- **Round trip.** For well-formed Kannada, converting to Nudi and back gives the same text. A property test checks this.
- **Unencodable characters.** Characters the layout has no glyph for, such as ಌ, ೄ, ೝ, the nukta, ZWJ, ZWNJ, Latin letters and spaces with no Windows-1252 byte, are kept in the output and listed in `unencodable` with their byte offsets. Aksharas that are not well formed, such as 2 vowel signs on 1 consonant, are listed in `malformed`. Kannada digits are written as ASCII digits; set the text in a `k` font to show Kannada digits.

## Segmentation

`aksharas(text)` splits text into aksharas and other segments. Joining the segments gives back the input exactly. The rules cover conjuncts of any length, nukta, ZWJ and ZWNJ after a virama, dead consonants at the end of a word, and Old Kannada letters. They are written out in [akshara.rs](../crates/kannadi/src/akshara.rs).

The legacy converter uses segmentation to check its own output.

## Normalization

`normalize(text)` gives the canonical form of the [Kannada text normalization profile](../NORMALIZATION.md): 1 spelling for text that renders the same. It applies Unicode NFC, builds ಊ, ಔ and ೠ from their parts, puts the nukta after its consonant, keeps a repeated mark once, and settles the joiners: ZWNJ after a virama shows the virama (ಕನ್‌ನಡ), ZWJ between ರ and its virama prevents the arkavattu (ರ‍್ಯ), other spellings of the same rendering become these, and joiners that change nothing are removed. In Kannada Wikipedia it removes 388,742 runs of joiners that do nothing, such as the ZWNJ in ಆಫ್‌, and rewrites 50,714 runs between a virama and a consonant, 36,376 of them a ZWJ used to show the virama.

`normalize::normalize_as(text, Form::Legacy)` also repairs converter and OCR mistakes: a zero or a Latin o typed for ಂ, and an arkavattu stored after its consonant (ಸಕರ್ಾರ for ಸರ್ಕಾರ). `Form::Search` also folds Old Kannada letters, joiners and the nukta, for search keys. `normalize_counted` counts the changes of each rule. [NORMALIZATION.md](../NORMALIZATION.md) lists every rule with its evidence, and [normalization/vectors.tsv](../normalization/vectors.tsv) has the test vectors.

```sh
kannadi normalize page.txt > page.normalized.txt
kannadi normalize --form legacy --stats converted.txt > clean.txt
kannadi normalize --check corpus/*.txt
```

`fix_zero_as_anusvara(text)` turns a Kannada zero (೦) written right after a letter into an anusvara (ಂ), even before a digit, as in ಕೊಠಡಿ೦೧. The legacy form keeps that zero.

`modernize(text)` maps ೞ to ಳ, ಱ to ರ and ೝ to ನ್. When a consonant follows ೝ, a ZWNJ keeps the ನ್ from joining it.

`typos::find(text)` suggests fixes for spellings that are valid Unicode but are often typing mistakes in modern Kannada. Sanskrit spells r + ṛ both ways below, so each finding is a suggestion for the writer, and nothing is changed in bulk. The rules:

- `ra-vocalic-r`: ರೃ (ra with the vowel sign ೃ) at the start of a syllable suggests ಋ, as in ನೈರೃತ್ಯ for ನೈಋತ್ಯ. Baraha phonetic typing produces it from `r~ru`.
- `ra-virama-vocalic-r`: ರ್ followed by ಋ suggests ಋ, as in ನೈರ್ಋತ್ಯ for ನೈಋತ್ಯ. Nudi typists draw ಋ with an arkavattu after it, which converts to ರ್ಋ. ರ್ before any other vowel letter, as in ಆರ್ಎನ್ಎ and ಕುರ್ಆನ್, is left alone, and so is ರ್ joined to ಋ by ZWNJ or ZWJ.

In Kannada Wikipedia (6.2 million lines), `ra-virama-vocalic-r` finds 1 spelling: ನಿರ್ಋತಿ, which Wikipedia writes ನಿಋತಿ 4 times elsewhere. It passes over 623 uses of ರ್ directly before another vowel letter, and 3,141 with ZWNJ or ZWJ between. In the Alar dictionary it finds 13, all Sanskrit-style headwords such as ನೈರ್ಋತಿ, which a dictionary keeps on purpose. The converter's 792 test lines have none.

## Transliteration

| Kannada | ISO 15919 | IAST | Baraha |
|---|---|---|---|
| ಕನ್ನಡ | kannaḍa | kannaḍa | kannaDa |
| ಬೆಂಗಳೂರು | beṁgaḷūru | beṃgaḷūru | beMgaLUru |
| ಸಂಸ್ಕೃತ | saṁskr̥ta | saṃskṛta | saMsk~ruta |
| ಏಳು | ēḷu | ēḷu | ELu |
| ಅಱಿವು | aṟivu | aṟivu | arxivu |
| ಮೞೆ | maḻe | maḻe | maLxe |

The full list of tested words is in [transliteration.tsv](../crates/kannadi/tests/fixtures/transliteration.tsv).

- **Exact round trips.** Kannada to Latin and back gives the same code points for Kannada text, including spaces, punctuation and joiners. The 1 exception is digits, below. Property tests check 2,000 random strings per run, and 50,000 in CI.
- **Separators.** When 2 letters would otherwise read as 1, a separator goes between them: `k:ha` for ಕ್ಹ and `ka:i` for ಕಇ in ISO 15919 and IAST. Baraha uses `~h` for ಹ after a consonant (`bakkiMg~hAm`), as Baraha's own documentation does, and `_` elsewhere.
- **Short and long e and o.** ಎ/ಏ and ಒ/ಓ stay apart in all 3 schemes.
- **IAST for Kannada.** Plain IAST was made for Sanskrit and has no letters for short e, short o, ಱ, ಳ or ೞ. This crate uses the ISO 15919 letters for those, and writes vocalic l as `l̥` because `ḷ` is ಳ.
- **Input.** Baraha accepts its alternative spellings (`aa` or `A`). ISO 15919 and IAST input is read without regard to case or Unicode composition.
- **Digits.** Kannada digits become ASCII digits in Latin output. Use `numerals::to_kannada_digits` to go the other way.
- **Pass-through.** Characters a scheme has no spelling for are copied as they are and still round-trip. Examples: ZWNJ in ISO 15919, nakaara pollu (ೝ), and nukta forms other than ಜ಼ and ಫ಼ in ISO 15919 and IAST.
- **Escapes.** A literal character that a scheme would read as part of a token is written with a backslash: `\'` for an apostrophe in ISO 15919 and IAST, `\~` `\@` `\^` in Baraha, and `\\` for a backslash. A `:` or `_` is a separator only between 2 letters, so `samaya: 10:30` stays as written.
- **Limits.** Latin to Kannada converts every Latin letter it can read, including English words.
- **IAST from Sanskrit sources.** In Sanskrit IAST, `e` and `o` are long vowels. This crate reads them as short ಎ and ಒ, so Sanskrit text in IAST comes out with short vowels.
- **Baraha `R`.** Current Baraha documentation uses `R` and `rx` for ಱ, and `~ru` for ಋ. Older Baraha text that writes ಋ as `Ru` (as in `kRuShNa`) reads as ಱು.

ISO 15919 output is cross-checked against [vidyut-lipi](https://github.com/ambuda-org/vidyut) on 5,000 random words per test run. The 2 crates differ on dead ಱ only: vidyut-lipi writes `r̆`, after the Marathi eyelash ra, and this crate writes `ṟ`. Nukta forms other than ಜ಼ and ಫ಼ are left out of the comparison.

## Old Kannada

ಱ, ೞ (which Unicode names "KANNADA LETTER FA"), the vowels ೠ, ಌ and ೡ with their signs, candrabindu, jihvamuliya and upadhmaniya are handled in segmentation and transliteration. Nakaara pollu (ೝ) is segmented as a syllable and passes through transliteration unchanged. The archaic vowel signs proposed to Unicode in 2014 are not encoded yet, so no software can support them.

## Architecture

The crate is split into small modules, each with 1 job:

| Module | Job |
|---|---|
| `chars` | Classifies characters in the Kannada block |
| `akshara` | Splits text into aksharas |
| `normalize` | The normalization profile's 3 forms, with counts per rule; Old Kannada modernization |
| `numerals` | Kannada digits: conversion, formatting, parsing |
| `phonetic` | Sound-alike keys for Kannada words and loose Latin spellings |
| `typos` | Well-formed spellings that are often typing mistakes, with suggested fixes |
| `translit` | 1 engine that romanizes and parses, driven by a table per scheme |
| `legacy` | Converts Nudi and Baraha glyph sequences to Unicode syllables and back, driven by the glyph table in `data/nudi.tsv` |

Scheme spellings and legacy glyph layouts are data tables read by 1 engine each. [AGENTS.md](../AGENTS.md) has the full guide for contributors.
