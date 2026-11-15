# fontqa

Shaping tests for Kannada fonts. fontqa shapes 2,146 Kannada strings with HarfBuzz in every font it is given and reports what breaks. The results for 41 Kannada fonts (146 faces) are in [results/](results/), described in [results/DATACARD.md](results/DATACARD.md). 35 of the fonts have an open license, 3 state no license, and 3 are Apple system fonts.

## What it tests

| Group | Strings | What a correct font does |
|---|---|---|
| syllable | 648: 36 consonants x 18 signs | Draws every consonant with every vowel sign, the virama, ಂ and ಃ |
| vowel | 16 | Draws every independent vowel, including ಌ, ೡ and ೠ |
| cluster | 1,123: consonant + virama + consonant, without ಱ and ೞ, and not starting with ರ (ರ್ರ is the exception) | Draws the second consonant as a subscript (ottu) |
| old_cluster | 140: clusters with ಱ or ೞ | The same, for Old Kannada letters |
| signed_cluster | 154: 10 common clusters with 15 signs, and 4 clusters of 3 consonants | The same, with a vowel sign, where marks meet subscripts |
| arkavattu | 54: ರ + virama + each modern consonant, ರ್ಕ with each sign, and 3 longer clusters | Draws ರ + virama as the arkavattu |
| rare | 11 | Draws ೝ, ೳ, U+0CDC, ೱ, ೲ, ಱ, ೞ, the nukta and the avagraha in context |

The 36 consonants are U+0C95 to U+0CB9 and ೞ U+0CDE. The 18 signs are the 13 dependent vowel signs, ೢ U+0CE2, ೣ U+0CE3, the virama, ಂ and ಃ. Signed clusters leave out ೄ, ೢ and ೣ, which do not occur on clusters in text. ೝ U+0CDD is a dead consonant, so it takes no vowel sign and is tested in the rare group. Each string is tested once. `fontqa cases` prints them all.

## What it detects

| Failure | How |
|---|---|
| `missing_glyph` | A character in the string has no glyph in `cmap`, or its glyph has no ink |
| `notdef` | The shaped output holds glyph 0 although every character is mapped |
| `dotted_circle` | HarfBuzz inserted U+25CC. A subfont maps U+25CC to an unused glyph id, so this is seen even when the font has no dotted circle |
| `no_conjunct` | The cluster shapes to the same glyphs as the same string with a ZWNJ after the virama, so no subscript formed |
| `no_arkavattu` | The same, for clusters starting with ರ, or the output is unchanged with the `rphf` feature turned off. Fonts shaped through an AAT `morx` table get only the ZWNJ test, since OpenType feature switches do not reach `morx` |
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
uv run fontqa check MyFont.ttf       # test any font file and print a summary; exits 1 only for
                                     # the failure kinds named with --fail-on
uv run fontqa fix In.ttf Out.ttf --knd2-blwf   # see "Fixes" below
uv run pytest
```

`fonts.toml` lists every font, where it comes from, and its license. `fonts.lock.json` holds the sha256 of every downloaded file. `fetch` refuses a downloaded or cached file that does not match it, and unpacks archives into a temporary folder first, so a failed run leaves nothing half cached. Font files are never committed. The macOS system fonts are read where they are installed and are not copied.

A proof sheet shows each failing string in the tested font, large, with Noto Sans Kannada below it as a reference. Collisions carry a red box around the overlap.

## Fixes

`fontqa fix` applies fixes that change no design:

- `--knd2-blwf` adds a `blwf` feature to the `knd2` script, with ligatures in the new Indic order (virama, consonant) built from the font's old-order rules. This fixes Gubbi.
- `--drop-aat` removes `morx`, `mort`, `feat` and `prop`. HarfBuzz uses a `morx` table in place of GSUB, so a broken `morx` breaks every conjunct even when GSUB is right. This fixes NudiParijatha.
- `--compose-rr` adds ೄ as ೃ plus the tail that turns ು into ೂ.
- `--compose-l` adds ೢ and ೣ: ಌ and ೡ scaled into the box of the font's own subscript ಲ, thickened to its stroke weight, with its mark class and mark positioning. Where ೡ is missing, it is built as ಌ plus the tail that turns ಋ into ೠ.
- Composing refuses a sign that already has a glyph with ink, and removes the `hdmx` and `LTSH` tables, which hold 1 entry per glyph.

The composed signs are drafts. A Kannada reader checks the proof sheet before they are used.

## Gates for drafted glyphs

`fontqa.gates` checks a drafted glyph against the font it goes into:

- `gate_strokes`: vertical stem, horizontal stroke and their ratio within 15% of the font's own glyphs, measured on scanlines;
- `gate_vertical`: the bottom and top ink edges within 20 units of theirs;
- `gate_contours`: closed contours with no tiny contours or short segments, the font's contour direction, and the expected number of counters with no slivers after overlap removal;
- `gate_compatible`: the same contours and points in every master of a variable font;
- `gate_shaping`: no test string fails that passed before.

Passing the gates means a draft is well formed and in proportion. It does not mean the shape is right; a reader decides that.
