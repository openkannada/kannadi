# Kannada font shaping dataset

Measured shaping results for open Kannada fonts, produced by fontqa.

## Contents

| File | Rows | What |
|---|---|---|
| `cases.csv` | 2,146 | Every test string: id, group, text, code points, and the ZWNJ reference string for clusters |
| `summary.csv` | 1 per face | Font metadata, license, source, sha256, and the count of each failure |
| `failures.csv` | 1 per failing string per face | The failures, the glyph names HarfBuzz produced, and detail for collisions and indistinct strings |
| `fonts/<face>.json` | 1 file per face | The summary, the font's own metadata, and its failures |

A string that does not appear in `failures.csv` for a face passed every check in that face. The full matrix is `cases.csv` crossed with the faces in `summary.csv`.

## Collection

- Run on 2026-11-16 with fontqa 0.1, HarfBuzz 15.0.0 (uharfbuzz 0.57.0), fontTools 4.68.0 and skia-pathops 0.9.2.
- 41 fonts, 146 faces after removing byte-identical files. 35 fonts have an open license (30 OFL 1.1, 5 GPL), 3 state no license, and 3 are Apple system fonts. The sources, pinned to commits where they are Git repositories, are in `../fonts.toml`; the file hashes are in `../fonts.lock.json`.
- Faces with fewer than 40 Kannada characters in `cmap` are left out. This drops the other scripts in Apple's `SFIndia.ttc`.
- Variable fonts are tested at their default instance only.

## Sources

Google Fonts (11 families), GitHub repositories of the designers or projects, the Brahmi project on SourceForge (Indian Institute of Science), the Debian package for Lohit Kannada, NLCI releases for Badami and Kaveri, KaGaPa's Nudi download for the Nudi Unicode fonts, and Sanchaya's showcase for 4 fonts that have no binary at their own source. The macOS system fonts are tested where macOS installs them.

## Licenses

The `license` column is what the font's own license file or name table says. Most are OFL 1.1. Gubbi and Navilu are GPL 3.0 with the font exception; Kedage and Mallige are GPL 2.0. Sampige, Cheluvi and the KaGaPa Nudi fonts state no license. Apple's fonts are proprietary. This dataset holds no font data, only test results and metadata.

## Limits

- The checks read HarfBuzz output. CoreText, DirectWrite and Uniscribe can shape the same font differently. In particular, CoreText uses a font's `morx` table, so NudiParijatha may work in macOS apps that use CoreText while it fails in browsers and on Android.
- `collision` flags ink overlap above 3% of the smaller glyph. Some fonts join strokes on purpose, so a collision needs a reader's check.
- `indistinct` flags identical ink. It cannot tell a deliberate merge from a mistake. Nishchaya Hosabaraha merges aspirated and unaspirated consonants on purpose, following the Hosa Baraha spelling reform.
- Spacing, kerning, legibility and design quality are not measured.
- The test set covers 2-consonant clusters, 10 common clusters with vowel signs, 4 clusters of 3 consonants, and 3 arkavattu clusters of 3 consonants. Longer clusters and real running text are not covered.
- Clusters with ಱ or ೞ are their own group, `old_cluster`. Most older fonts fail only these, and they matter for Old Kannada text alone.
- For fonts shaped through an AAT `morx` table, such as Apple's Kannada MN, the arkavattu test is the ZWNJ comparison alone.

## Reproduce

```sh
cd tools/fontqa
uv sync --locked
uv run fontqa fetch
uv run fontqa run
```

## License

The test strings, scripts and computed results are MIT licensed, like the rest of this repository. The font names, versions and copyright strings recorded from each font belong to their owners and are kept only to identify the font tested.
