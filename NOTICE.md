# Notice

kannadi
Copyright 2026 The Open Kannada Authors

The code and the glyph tables are under the MIT license or the Apache License 2.0, at your choice ([LICENSE-MIT](LICENSE-MIT), [LICENSE-APACHE](LICENSE-APACHE)). The docs are under CC BY 4.0 ([LICENSES/CC-BY-4.0.txt](LICENSES/CC-BY-4.0.txt)). [REUSE.toml](REUSE.toml) gives the license of every file. If you redistribute kannadi, keep this notice and the license files.

## Third-party material in this repository

| Material | Where | License | Credit |
|---|---|---|---|
| Noto Sans Kannada and Noto Serif Kannada, subset to WOFF2 | `web/src/fonts/` | OFL-1.1 (`web/src/fonts/OFL.txt`) | Copyright 2022 The Noto Project Authors, [notofonts/kannada](https://github.com/notofonts/kannada) |
| Short excerpts of Karnataka government web pages (2002), Karnataka Acts (2001 to 2003), budget papers (2002 to 2014) and Legislative Assembly member pages | `corpus/nudi-2002.tsv`, `crates/kannadi/tests/fixtures/{nudi,akruti,nudi-b,kan-eng}.tsv`, and a few words in tests | No license stated; not covered by this repository's licenses ([LICENSES/LicenseRef-Excerpts.txt](LICENSES/LicenseRef-Excerpts.txt)) | Government of Karnataka; each line records its source URL |
| Font names, versions and copyright strings read from the fonts tested | `tools/fontqa/results/` | Belong to their owners; recorded as facts about each font | Each font's source is in `tools/fontqa/fonts.toml` |

## Used, not included

- Rust crates listed in `Cargo.lock`, fetched by Cargo, under their own licenses. All are permissive (MIT, Apache-2.0, BSD, Zlib, Unicode-3.0). [vidyut-lipi](https://github.com/ambuda-org/vidyut) (MIT) is used only as a test oracle.
- Python packages listed in `tools/fontqa/uv.lock`, under their own licenses.
- The fonts that `fontqa` tests are downloaded at run time from the sources in `tools/fontqa/fonts.toml` and checked by hash. No font file of theirs is committed.

## Glyph tables

The tables in `crates/kannadi/data/` record which glyph code each legacy font uses for which Kannada letter. They were derived by rendering the fonts and reading public documents, and each table's header says how. They hold no font software: no font files, outlines or hints. The code assignments are facts that any converter needs for compatibility. Nudi, Baraha, Akruti, Shree-Lipi, Shree-Kan-0850, Nudi B-Akshara and Kan-Eng are names of their owners' fonts and products, used here only to name the layouts.

No code or table was copied from other converters. In particular, nothing from ascii2unicode (GPL-3.0) is included.

The Shree-Lipi test lines in `crates/kannadi/tests/fixtures/shree-lipi.tsv` were written for kannadi and encoded with its own table.

## Removal requests

To ask for an excerpt to be removed, write to support@luminik.io or open an issue.
