# Benchmark corpus: data card

`nudi-2002.tsv` holds 207 short lines of Kannada typed in Baraha ANSI fonts on Karnataka government web pages in 2002, with the glyph codes as stored and the expected Unicode. It measures the legacy converter in both directions.

## Contents

| | |
|---|---|
| Lines | 207, of 2 to 11 words, 7.4 on average; 1,540 words in all |
| Documents | 31 pages from 2 sources |
| Fonts | BRH Kannada, BRH Sirigannada and other BRH fonts, which share the Nudi layout |
| Years | 2002 |
| Status | 100 lines in the verification set, pending a Kannada reader's check; 107 unchecked; 0 verified |

Each line has 6 tab-separated columns: an id, a status, the source URL, the glyph codes as Windows-1252 bytes in hex, the expected Unicode, and a note.

- **verified**: a Kannada reader compared the expected text with the line as drawn in its own font.
- **pending**: in the 100-line verification set, which shows each line drawn in the Nudi 01 e font next to the converter's output. The expected text is the converter's output until the check is done. The note names the line in the verification set and any question raised for it.
- **unchecked**: the expected text is the converter's output and nobody has reviewed it.

No line is marked verified until a reader has checked it.

## Sources

All pages were fetched from the Internet Archive. Each line records the archived URL with its timestamp.

| Source | Pages | Archived |
|---|---|---|
| Rural Development and Panchayat Raj department, scheme pages (`kar.nic.in/rdpr/kan-www/`) | 24 | December 2002 |
| Finance department, 2002 budget pages (`kar.nic.in/finance/bud2002-kan/`) | 7 | 2004 to 2009 copies of the 2002 pages |

## How lines were chosen

1. The pages were converted with `kannadi to-unicode`, which converts only text set in a legacy font.
2. Each word of the original page was paired with the same word of the converted page. Words the converter changed are legacy-font text.
3. The 100 lines of the verification set were found in their pages by matching the converted words. 4 lines from the budget table skip the numbers between table cells, which are in an English font. 1 line, the first, joins a heading with the first sentence after it, as in `crates/kannadi/tests/fixtures/nudi.tsv`.
4. Up to 4 more lines were taken from each page: runs of 8 legacy-font words that convert with no unknown glyph or malformed syllable, at least 40 words apart, and not in the verification set.

Lines keep the source's own typing, including its mistakes.

## How it is used

- `cargo test -p kannadi --test corpus` checks that every line converts to its expected text, and that every line but 103 and 166 encodes back to the bytes that were typed.
- `cargo run -p kannadi --example corpus` prints, per status, the lines that convert exactly, the akshara error rate, lines with problems, and lines that encode as typed.

Today, with no verified lines, the figures compare the converter with itself. They become an accuracy measure as lines are verified. When a reader finds an error, the expected text is corrected, the line is marked verified, and the test fails until the converter or its table is fixed.

## License and use

The pages are works of the Government of Karnataka. They carry no license statement, and the MIT license of this repository does not cover them. The corpus keeps short excerpts, 1 to 21 lines per page and 6.7 on average, for research on and testing of a converter, with the source of each line. No page is stored in full. Keeping short excerpts with their sources for research and testing is meant as fair dealing under the Indian Copyright Act, 1957.

To ask for an excerpt to be removed, open an issue or write to prasadus92@gmail.com.

## Known gaps

- 1 year, 2 departments and 1 font family. There are no Nudi-proper documents, no DOCX or PDF sources, and nothing after 2002.
- Questions raised for the verification set and not yet answered:
  - Glyph D2 is read as subscript ಛ. In ವಿಜ್ಞಾನ it may stand for subscript ಞ (lines 38, 48, 51, 57 and 82 of the check set).
  - The ೂ drawn in ವೊ and ಮೊ look alike (lines 29 and 100).
  - Glyph F5 is unknown (line 3).
- 2 unchecked lines do not encode back to their typed bytes, because of typing habits: in line 103, ರ್ and ನ್ are typed as syllables of their own, and line 166 has an extra spacing glyph.
- English typed in a Kannada font, which some pages contain, is left out of the line selection.
