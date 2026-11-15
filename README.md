# kannadi

Tools for Kannada text in Rust. The goal is to convert documents typed in old Nudi and Baraha fonts to Unicode, and back, with checked accuracy.

ಕನ್ನಡಿ (kannaḍi) means mirror.

## Why

For years, Kannada text in Karnataka was typed with Nudi and Baraha fonts that store glyph codes in place of letters. Government orders, court records and newspaper archives from that time cannot be searched, indexed or read aloud until they are converted. The open converters available today disagree with each other, and none publishes a verified test set.

## Status

Version 0.1, in development.

| Part | State |
|---|---|
| Akshara segmentation | Done |
| Normalisation and Old Kannada modernisation | Done |
| Transliteration: ISO 15919, IAST, Baraha phonetic | Done |
| Kannada numerals | Done |
| Nudi and Baraha to Unicode, and back | Designed: [docs/legacy-converter.md](docs/legacy-converter.md) |
| Verified test corpus | Planned: [docs/corpus-plan.md](docs/corpus-plan.md) |
| Converter page that runs in the browser, with no upload | Planned |
| Command-line tool for TXT, HTML and DOCX | Planned |

## Example

```rust
use kannadi::{aksharas, modernise, normalize, numerals, transliterate, Scheme};

// Aksharas: the units a reader sees as 1 letter.
let parts: Vec<&str> = aksharas("ಸ್ತ್ರೀಯರು").map(|a| a.text).collect();
assert_eq!(parts, ["ಸ್ತ್ರೀ", "ಯ", "ರು"]);

// Normalisation fixes text that looks right but compares wrong.
assert_eq!(normalize("ಬೆ೦ಗಳೂರು"), "ಬೆಂಗಳೂರು"); // digit zero typed for anusvara

// Old Kannada to modern spelling, for search.
assert_eq!(modernise("ಅಱಿವು ಬಾೞ್"), "ಅರಿವು ಬಾಳ್");

// Transliteration, both ways.
assert_eq!(transliterate("ಕನ್ನಡ ನಾಡು", Scheme::Kannada, Scheme::Iso15919), "kannaḍa nāḍu");
assert_eq!(transliterate("kannaDa nADu", Scheme::Baraha, Scheme::Kannada), "ಕನ್ನಡ ನಾಡು");

// Numerals.
assert_eq!(numerals::format(1956), "೧೯೫೬");
```

Run `cargo run --example demo` to see each feature on 1 sentence.

## Segmentation

`aksharas(text)` splits text into aksharas and other segments. Joining the segments gives back the input exactly. The rules cover conjuncts of any length, nukta, ZWJ and ZWNJ after a virama, dead consonants at the end of a word, and Old Kannada letters. They are written out in [akshara.rs](crates/kannadi/src/akshara.rs).

The legacy converter uses segmentation to check its own output.

## Normalisation

`normalize(text)` applies Unicode NFC, then 3 Kannada fixes:

1. ಉ + ಾ becomes ಊ, and ಋ + ಾ becomes ೠ. They look the same, and Unicode says to use the single letter.
2. A Kannada zero (೦) right after a letter becomes an anusvara (ಂ). Legacy fonts and OCR often mix them up.
3. A mark typed twice in a row (್್, ಂಂ) is kept once.

`modernise(text)` maps ೞ to ಳ, ಱ to ರ and ೝ to ನ್.

## Transliteration

| Kannada | ISO 15919 | IAST | Baraha |
|---|---|---|---|
| ಕನ್ನಡ | kannaḍa | kannaḍa | kannaDa |
| ಬೆಂಗಳೂರು | beṁgaḷūru | beṃgaḷūru | beMgaLUru |
| ಸಂಸ್ಕೃತ | saṁskr̥ta | saṃskṛta | saMsk~ruta |
| ಏಳು | ēḷu | ēḷu | ELu |
| ಅಱಿವು | aṟivu | aṟivu | arxivu |
| ಮೞೆ | maḻe | maḻe | maLxe |

The full list of tested words is in [transliteration.tsv](crates/kannadi/tests/fixtures/transliteration.tsv).

- **Exact round trips.** Kannada to Latin and back gives the same code points, for any sequence of Kannada characters. Property tests check 2,000 random strings per run, and 50,000 in CI.
- **Separators.** When 2 letters would otherwise read as 1, a separator goes between them: `k:ha` for ಕ್ಹ and `ka:i` for ಕಇ in ISO 15919 and IAST. Baraha uses `~h` for ಹ after a consonant (`bakkiMg~hAm`), as Baraha's own documentation does, and `_` elsewhere.
- **Short and long e and o.** ಎ/ಏ and ಒ/ಓ stay apart in all 3 schemes.
- **IAST for Kannada.** Plain IAST was made for Sanskrit and has no letters for short e, short o, ಱ, ಳ or ೞ. This crate uses the ISO 15919 letters for those, and writes vocalic l as `l̥` because `ḷ` is ಳ.
- **Input.** Baraha accepts its alternative spellings (`aa` or `A`). ISO 15919 and IAST input is read without regard to case or Unicode composition.
- **Digits.** Kannada digits become ASCII digits in Latin output. Use `numerals::to_kannada_digits` to go the other way.
- **Pass-through.** Characters a scheme has no spelling for, such as ZWNJ in ISO 15919, are copied as they are and still round-trip.
- **Limits.** Latin to Kannada converts every Latin letter it can read, including English words. A literal `:` or `'` between letters in Kannada text is read back as a separator or avagraha.

ISO 15919 output is cross-checked against [vidyut-lipi](https://github.com/ambuda-org/vidyut) on 5,000 random words per test run. The 2 crates differ on dead ಱ only: vidyut-lipi writes `r̆`, after the Marathi eyelash ra, and this crate writes `ṟ`. Nukta forms other than ಜ಼ and ಫ಼ are left out of the comparison.

## Old Kannada

ಱ, ೞ (which Unicode names "KANNADA LETTER FA"), nakaara pollu, the vowels ೠ, ಌ and ೡ with their signs, candrabindu, jihvamuliya and upadhmaniya are handled in segmentation and transliteration. The archaic vowel signs proposed to Unicode in 2014 are not encoded yet, so no software can support them.

## Prior art

See [docs/prior-art.md](docs/prior-art.md).

## Development

```sh
cargo test
PROPTEST_CASES=50000 cargo test --release --test props
cargo clippy --all-targets
```

The minimum supported Rust version is 1.75.

## Licence

MIT. See [LICENSE](LICENSE).

If kannadi helps your project, a mention or a link to [prasad.tech](https://prasad.tech) is appreciated.
