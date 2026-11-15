# fontqa

Shaping tests for Kannada fonts. fontqa shapes 1,905 Kannada strings with HarfBuzz in every font it is given and reports what breaks. The results for 41 open Kannada fonts (146 faces) are in [results/](results/), described in [results/DATACARD.md](results/DATACARD.md).

## What it tests

| Group | Strings | What a correct font does |
|---|---|---|
| syllable | 576: 36 consonants x 16 signs | Draws every consonant with every vowel sign and the virama |
| cluster | 1,260: consonant + virama + consonant | Draws the second consonant as a subscript (ottu) |
| arkavattu | 36 clusters starting with ರ, plus 19 more | Draws ರ + virama as the arkavattu after the cluster |
| rare | 14 | Draws ೝ, ೳ, U+0CDC, ೱ, ೲ, ೢ, ೣ, ಌ, ೡ, ೄ, ೠ, ಱ and ೞ in context |

The 36 consonants are U+0C95 to U+0CB9 and ೞ U+0CDE. The 16 signs are the 13 dependent vowel signs, ೢ U+0CE2, ೣ U+0CE3 and the virama. ೝ U+0CDD is a dead consonant, so it takes no vowel sign and is tested in the rare group. `fontqa cases` prints every string.

## What it detects

| Failure | How |
|---|---|
| `missing_glyph` | A character in the string has no glyph in `cmap`, or its glyph has no ink |
| `notdef` | The shaped output holds glyph 0 although every character is mapped |
| `dotted_circle` | HarfBuzz inserted U+25CC. A subfont maps U+25CC to an unused glyph id, so this is seen even when the font has no dotted circle |
| `no_conjunct` | The cluster shapes to the same glyphs as the same string with a ZWNJ after the virama, so no subscript formed |
| `no_arkavattu` | The same, for clusters starting with ರ, or the nominal ರ glyph is still in the output |
| `collision` | 2 glyphs other than the base, at least 1 of them a mark, overlap by more than 3% of the smaller glyph's ink. Ink overlap is measured on the outlines with skia-pathops, after a bounding box test |
| `indistinct` | The string renders with exactly the same ink as a different test string, for example ೄ drawn as ೃ |

Collisions are candidates for a reader to check. Some designs join strokes on purpose.

Variable fonts are tested at their default instance.

## Use

```sh
cd tools/fontqa
uv sync
uv run fontqa fetch                  # download the fonts in fonts.toml to ~/.cache/kannadi-fontqa
uv run fontqa run --proofs /tmp/p    # test them all; write results/ and 1 proof sheet per face
uv run fontqa check MyFont.ttf       # test any font file and print a summary
uv run fontqa fix In.ttf Out.ttf --knd2-blwf   # see "Fixes" below
uv run pytest
```

`fonts.toml` lists every font, where it comes from, and its license. `fonts.lock.json` holds the sha256 of every downloaded file; `fetch` refuses a file that changed. Font files are never committed. The macOS system fonts are read where they are installed and are not copied.

A proof sheet shows each failing string in the tested font, large, with Noto Sans Kannada below it as a reference. Collisions carry a red box around the overlap.

## Fixes

`fontqa fix` applies fixes that change no design:

- `--knd2-blwf` adds a `blwf` feature to the `knd2` script, with ligatures in the new Indic order (virama, consonant) built from the font's old-order rules. This fixes Gubbi.
- `--drop-aat` removes `morx`, `mort`, `feat` and `prop`. HarfBuzz uses a `morx` table in place of GSUB, so a broken `morx` breaks every conjunct even when GSUB is right. This fixes NudiParijatha.
- `--compose-rr` adds ೄ as ೃ plus the tail that turns ು into ೂ.
- `--compose-l` adds ೢ and ೣ: ಌ and ೡ scaled into the box of the font's own subscript ಲ, thickened to its stroke weight, with its mark class and mark positioning. Where ೡ is missing, it is built as ಌ plus the tail that turns ಋ into ೠ.

The composed signs are drafts. A Kannada reader checks the proof sheet before they are used.
