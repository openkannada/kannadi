# kannadi

Kannada text tools in Rust: akshara segmentation, normalisation, transliteration and numerals, with support for Old Kannada letters.

ಕನ್ನಡಿ (kannaḍi) means mirror. The library turns Kannada text into other forms and back without losing anything.

## Status

Version 0.1, in development. Not yet published to crates.io.

| Part | State |
|---|---|
| Akshara segmentation | Done |
| Normalisation and modernisation | Done |
| Transliteration: ISO 15919, IAST, Baraha phonetic | Done |
| Kannada numerals | Done |
| Nudi and Baraha legacy fonts to Unicode, and back | Designed, see [docs/legacy-converter.md](docs/legacy-converter.md) |
| Command-line tool for TXT, HTML and DOCX | Planned |
| WebAssembly, npm package and an in-browser converter page | Planned |
| Python bindings | Planned |

## Example

```rust
use kannadi::{aksharas, modernise, normalize, numerals, transliterate, Scheme};

// Transliteration, both ways.
assert_eq!(transliterate("ಕನ್ನಡ ನಾಡು", Scheme::Kannada, Scheme::Iso15919), "kannaḍa nāḍu");
assert_eq!(transliterate("kannaDa nADu", Scheme::Baraha, Scheme::Kannada), "ಕನ್ನಡ ನಾಡು");

// Aksharas: the units a reader sees as one letter.
let parts: Vec<&str> = aksharas("ಸ್ತ್ರೀಯರು").map(|a| a.text).collect();
assert_eq!(parts, ["ಸ್ತ್ರೀ", "ಯ", "ರು"]);

// Normalisation fixes text that looks right but compares wrong.
assert_eq!(normalize("ಬೆ೦ಗಳೂರು"), "ಬೆಂಗಳೂರು"); // digit zero typed for anusvara

// Old Kannada to modern spelling, for search.
assert_eq!(modernise("ಅಱಿವು ಬಾೞ್"), "ಅರಿವು ಬಾಳ್");

// Numerals.
assert_eq!(numerals::format(1956), "೧೯೫೬");
```

Run `cargo run --example demo` to see every feature on one sentence.

## Transliteration

| Kannada | ISO 15919 | IAST | Baraha |
|---|---|---|---|
| ಕನ್ನಡ | kannaḍa | kannaḍa | kannaDa |
| ಬೆಂಗಳೂರು | beṁgaḷūru | beṃgaḷūru | beMgaLUru |
| ಸಂಸ್ಕೃತ | saṁskr̥ta | saṃskṛta | saMsk~ruta |
| ಏಳು | ēḷu | ēḷu | ELu |
| ಅಱಿವು | aṟivu | aṟivu | arxivu |
| ಮೞೆ | maḻe | maḻe | maLxe |

The full list of tested words is in [crates/kannadi/tests/fixtures/transliteration.tsv](crates/kannadi/tests/fixtures/transliteration.tsv).

Rules worth knowing:

- **Round trips are exact.** Kannada to Latin and back gives the same code points, for any sequence of Kannada characters. Property tests check this on 2,000 random strings per run, and 50,000 in CI.
- **Separators.** When two letters would otherwise read as one, a separator goes between them: `k:ha` for ಕ್ಹ and `ka:i` for ಕಇ in ISO 15919 and IAST. Baraha uses `~h` for ಹ after a consonant (`bakkiMg~hAm`), as Baraha's own documentation does, and `_` elsewhere.
- **Short and long e and o.** All three schemes keep ಎ/ಏ and ಒ/ಓ apart: `e`/`ē` and `o`/`ō` in ISO 15919 and IAST, `e`/`E` and `o`/`O` in Baraha.
- **IAST for Kannada.** Plain IAST was made for Sanskrit, which has no short e, short o, ಱ, ಳ or ೞ. This crate uses the ISO 15919 letters for those, and writes vocalic l as `l̥` because `ḷ` is ಳ.
- **Baraha input aliases.** Baraha accepts several spellings for some letters (`aa` or `A`, `K` or `kh`). All are read; output uses one spelling.
- **Input is forgiving.** ISO 15919 and IAST input is read without regard to case or Unicode composition, and each accepts the other's spellings of ಂ and ಋ.
- **Digits.** Kannada digits become ASCII digits in Latin output. ASCII digits stay ASCII on the way back; use `numerals::to_kannada_digits` if you want Kannada digits.
- **What passes through.** Characters with no spelling in a scheme, such as ZWNJ in ISO 15919 or nakaara pollu (ೝ), are copied as they are, so they still round-trip.
- **Limits.** Latin to Kannada converts every Latin letter it can read, so English words in the input are converted too. A literal `:` or `'` between letters in Kannada text is read back as a separator or avagraha.

ISO 15919 output is cross-checked against [vidyut-lipi](https://github.com/ambuda-org/vidyut) on 5,000 random words per test run, separators included. The one difference is dead ಱ, which vidyut-lipi writes `r̆` (the Marathi eyelash ra) and this crate writes `ṟ`. Nukta forms other than ಜ಼ and ಫ಼ are not compared: vidyut-lipi uses Urdu letters such as `q` for ಕ಼, and this crate passes the nukta through.

## Segmentation

`aksharas(text)` splits text into aksharas and other segments. Joining the segments always gives back the input. The rules handle conjuncts of any length, nukta, ZWJ and ZWNJ after a virama, dead consonants at the end of a word, and Old Kannada letters. They are written out in [crates/kannadi/src/akshara.rs](crates/kannadi/src/akshara.rs).

## Normalisation

`normalize(text)` applies Unicode NFC and then 3 Kannada-specific fixes:

1. ಉ + ಾ becomes ಊ, and ಋ + ಾ becomes ೠ. They look the same, and Unicode says to use the single letter.
2. A Kannada zero (೦) right after a letter becomes an anusvara (ಂ). Legacy fonts and OCR often mix them up.
3. A mark typed twice in a row (್್, ಂಂ) is kept once.

`modernise(text)` maps ೞ to ಳ, ಱ to ರ and ೝ to ನ್.

## Old Kannada

ಱ (RRA), ೞ (LLLA, which Unicode misnames "KANNADA LETTER FA"), nakaara pollu, the vowels ೠ, ಌ and ೡ with their signs, candrabindu, jihvamuliya and upadhmaniya are handled in segmentation and transliteration. Archaic vowel signs that are not yet in Unicode cannot be supported until they are encoded.

## Prior art

- [vidyut-lipi](https://github.com/ambuda-org/vidyut) (MIT): a general Indic transliterator built for Sanskrit. Used here as a test oracle.
- [lipilekhika](https://github.com/shubhattin/lipilekhika) (MIT) and [shlesha](https://github.com/udapaana/shlesha) (MIT or Apache 2.0): general Indic transliterators.
- [ascii2unicode](https://github.com/aravindavk/ascii2unicode) by Aravinda VK (GPL 3.0 or later): the existing open-source Nudi and Baraha converter, in Python and JavaScript.

This crate focuses on Kannada only: Kannada-specific spelling rules, Old Kannada, and legacy font conversion. See [docs/prior-art.md](docs/prior-art.md) for the comparison.

## Development

```sh
cargo test                                   # unit, golden, property and differential tests
PROPTEST_CASES=50000 cargo test --release --test props
cargo clippy --all-targets
```

The minimum supported Rust version is 1.75.

## Licence

MIT or Apache 2.0, at your option.
