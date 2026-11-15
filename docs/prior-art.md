# Prior art

Checked on 15 November 2026.

## Rust transliterators

| Crate | Licence | Kannada | Notes |
|---|---|---|---|
| [vidyut-lipi](https://github.com/ambuda-org/vidyut) 0.2.0 | MIT | Yes, through a Devanagari pivot | Built for Sanskrit. Has ISO 15919, IAST and Baraha South tables, including ಱ and ೞ. Pulls in `clap`, `wasm-bindgen` and `serde-wasm-bindgen` as normal dependencies. |
| [lipilekhika](https://github.com/shubhattin/lipilekhika) 1.2.0 | MIT | Yes | Typing and conversion tool for many Indian scripts. |
| [shlesha](https://github.com/udapaana/shlesha) 0.5.7 | MIT or Apache 2.0 | Yes | Built for Vedic Sanskrit, with a focus on speed. |

### What was reused and why

All 3 licences would allow reuse. This crate still writes its own Kannada tables, for 4 reasons:

1. **Kannada rules differ from Sanskrit rules.** Short e and o are everyday letters in Kannada; IAST as used for Sanskrit has no spelling for them. Kannada also needs ಳ, ಱ and ೞ in every scheme.
2. **Lossless round trips.** This crate guarantees that any Kannada string survives Kannada to Latin and back. That needs separators chosen per scheme and pass-through for unspelled characters, which is easier to prove on a small, single-script engine.
3. **Size.** The browser build should stay small. Each Kannada table here has under 100 entries.
4. **Legacy fonts.** None of the 3 handle Nudi or Baraha glyph encodings, which is the main gap this project fills.

vidyut-lipi is used as a **test oracle**: a property test generates 5,000 random Kannada words per run and checks that both crates give the same ISO 15919 output, separators included (`k:ha` for ಕ್ಹ, `ka:i` for ಕಇ). Known, deliberate differences:

- Dead ಱ (ಱ್): vidyut-lipi writes `r̆`, from the Marathi eyelash ra in its Devanagari pivot. This crate writes `ṟ`, the ISO 15919 letter for Dravidian RRA.
- Nukta forms other than ಜ಼ (z) and ಫ಼ (f): vidyut-lipi uses the ISO 15919 letters for Urdu sounds (`q` for ಕ಼). This crate passes the nukta through unchanged. These are left out of the comparison.

## Baraha phonetic scheme

The Baraha tables follow Baraha's own Kannada keyboard help page (`baraha.com/help/Keyboards/kan-phonetic.htm`). Where it differs from older Baraha spellings found elsewhere, the help page wins: `~ru` for ಋ (not `Ru`), `rx` or `R` for ಱ, `Lx` for ೞ, `@av` for avagraha, `^` and `^^` for ZWJ and ZWNJ. Baraha's documented way to stop ಹ joining a consonant is `~h` (`bakkiMg~hAm`). Baraha documents no general separator; this crate uses `_`, as vidyut-lipi does.

## Legacy converters

| Project | Licence | Language | State |
|---|---|---|---|
| [aravindavk/ascii2unicode](https://github.com/aravindavk/ascii2unicode) | GPL 3.0 or later (in `COPYING`; GitHub shows "Other" because it does not detect it) | Python CLI, JavaScript web page | Last push 8 August 2023. Linux only for the CLI. One mapping table for Nudi and Baraha. |
| Google Docs add-ons and web pages | Closed | | |

ascii2unicode is the reference. Its code is not copied here: GPL code cannot go into an MIT or Apache crate without the author's permission. The design in [legacy-converter.md](legacy-converter.md) is written from the problem, the fonts and test documents. A message proposing collaboration has been drafted for its author.
