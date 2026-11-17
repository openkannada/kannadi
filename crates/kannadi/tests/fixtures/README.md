# Test fixtures

Golden fixtures for the legacy converter and for transliteration. Each legacy fixture line has the glyph codes as Windows-1252 bytes in hex, the expected Unicode, and its source.

| File | Lines | Text | License |
|---|---|---|---|
| `nudi.tsv` | 12 | Karnataka government web pages, 2002 | Excerpt, see below |
| `akruti.tsv` | 125 | Karnataka Acts of 2001 to 2003 and the 2002 budget highlights | Excerpt, see below |
| `nudi-b.tsv` | 190 | 13th Karnataka Legislative Assembly member pages and Karnataka budget papers | Excerpt, see below |
| `kan-eng.tsv` | 118 | Karnataka budget papers, 2002 to 2011 | Excerpt, see below |
| `shree-lipi.tsv` | 140 | Written for kannadi and encoded with its Shree-Lipi table | MIT OR Apache-2.0 |
| `transliteration.tsv` | 38 | Written for kannadi | MIT OR Apache-2.0 |

## Government excerpts

Source credit: Government of Karnataka. The pages and PDFs were fetched from the Internet Archive, and each line records the archived URL of its source.

They carry no license statement, and the licenses of this repository do not cover them. They are kept as short excerpts for research on and testing of converters, as fair dealing under the Indian Copyright Act, 1957. No document is stored in full.

To ask for an excerpt to be removed, write to support@luminik.io or open an issue.

## Writing new lines

Do not add newspaper or other commercial text. For a layout with no public government documents, write the lines yourself, encode them with the layout's table, and retype words by hand where the test needs a typing habit the encoder does not write. `shree-lipi.tsv` was made this way.
