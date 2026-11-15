# Prior art

Checked on 15 November 2026.

## Rust transliterators

| Crate | License | Kannada | Notes |
|---|---|---|---|
| [vidyut-lipi](https://github.com/ambuda-org/vidyut) 0.2.0 | MIT | Yes, through a Devanagari pivot | Built for Sanskrit. Has ISO 15919, IAST and Baraha South tables, including ಱ and ೞ. Pulls in `clap`, `wasm-bindgen` and `serde-wasm-bindgen` as normal dependencies. |
| [lipilekhika](https://github.com/shubhattin/lipilekhika) 1.2.0 | MIT | Yes | Typing and conversion tool for many Indian scripts. |
| [shlesha](https://github.com/udapaana/shlesha) 0.5.7 | MIT or Apache 2.0 | Yes | Built for Vedic Sanskrit, with a focus on speed. |

### What was reused and why

All 3 licenses would allow reuse. This crate still writes its own Kannada tables, for 4 reasons:

1. **Kannada rules differ from Sanskrit rules.** Short e and o are everyday letters in Kannada; IAST as used for Sanskrit has no spelling for them. Kannada also needs ಳ, ಱ and ೞ in every scheme.
2. **Lossless round trips.** This crate guarantees that any Kannada string survives Kannada to Latin and back. That needs separators chosen per scheme and pass-through for unspelled characters, which is easier to prove on a small, single-script engine.
3. **Size.** The browser build should stay small. Each Kannada table here has under 100 entries.
4. **Legacy fonts.** None of the 3 handle Nudi or Baraha glyph encodings, which is the main gap this project fills.

vidyut-lipi is used as a **test oracle**: a property test generates 5,000 random Kannada words per run and checks that both crates give the same ISO 15919 output, separators included (`k:ha` for ಕ್ಹ, `ka:i` for ಕಇ). Known, deliberate differences:

- Dead ಱ (ಱ್): vidyut-lipi writes `r̆`, from the Marathi eyelash ra in its Devanagari pivot. This crate writes `ṟ`, the ISO 15919 letter for Dravidian RRA.
- Nukta forms other than ಜ಼ (z) and ಫ಼ (f): vidyut-lipi uses the ISO 15919 letters for Urdu sounds (`q` for ಕ಼). This crate passes the nukta through unchanged. These are left out of the comparison.

## Baraha phonetic scheme

The Baraha tables follow Baraha's own Kannada keyboard help page (`baraha.com/help/Keyboards/kan-phonetic.htm`). Where it differs from older Baraha spellings found elsewhere, the help page wins. It uses `~ru` for ಋ (older lists use `Ru`), `rx` or `R` for ಱ, `Lx` for ೞ, `@av` for avagraha, `^` and `^^` for ZWJ and ZWNJ. Baraha's documented way to stop ಹ joining a consonant is `~h` (`bakkiMg~hAm`). Baraha documents no general separator, so this crate uses `_`, as vidyut-lipi does.

## Legacy converters

| Project | License | Language | Last activity |
|---|---|---|---|
| [aravindavk/ascii2unicode](https://github.com/aravindavk/ascii2unicode) | GPL 3.0 or later, stated in `COPYING` | Python CLI for Linux, JavaScript web page | August 2023 |
| [kagapa-blr/KannadaAsciiUnicodeSDK](https://github.com/kagapa-blr/KannadaAsciiUnicodeSDK) | No license file | C# | August 2026 |
| [kannada-ascii-unicode](https://pypi.org/project/kannada-ascii-unicode/) | MIT | Python | July 2026 |
| Google Docs add-ons, Baraha's converter, web pages | Closed | | |

None of them publishes a verified test set or accuracy figures. This crate derives its own tables from the font files and real documents, as described in [legacy-converter.md](legacy-converter.md). No code or tables from these projects are used.
