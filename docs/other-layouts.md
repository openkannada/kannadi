# Layouts with no table yet

Status: checked on 17 November 2026.

kannadi converts 5 legacy layouts: Nudi (with Baraha's ANSI fonts), Nudi B-Akshara, Kan-Eng, Akruti and Shree-Lipi. This page lists the other layouts found in public documents, what is known about each, and what a table needs.

## What a table needs

- At least 100 lines of public text in the layout. Fewer lines cannot check the glyphs a table claims. No table is built below that.
- The font, as its maker or a publisher served it with public pages, or embedded in a public PDF. See the rules in [shree-lipi.md](shree-lipi.md#rules-for-deriving-a-layout-table).

## Waiting for documents

These layouts have too little public text for a table. They wait for more documents.

| Font | Maker | Where it was found | Lines found | State |
|---|---|---|---|---|
| AkliteKndPadmini | Akruti family | The Karnataka Repealing and Amending Act of 2003, [PDF](https://web.archive.org/web/20090409222424/http://dpal.kar.nic.in/ao2003/Repealing%20and%20Amending%20Act-K.pdf). No other public document was found. | 19 | Waiting for documents |
| KN-Nandi (KN-Nandi-Normal, KN-Nandi-Bold) | C-DAC | SSLC model answers of 2019 and 2020 from the Karnataka Secondary Education Examination Board, such as [14E-B](https://web.archive.org/web/20200215100603/http://kseeb.kar.nic.in:80/.%5Cdocs%5CMODEL-ANSWERS-2019%5CFIRST%20LANGUAGE%5C14E-B%20Version.pdf), as Type 1 subsets with Mac glyph names | 15 distinct lines in 3 PDFs | Waiting for documents |

The command-line tool reports text in these fonts as `not converted`, with the text left out, so nothing is converted in the wrong layout.

The board's archive holds about 440 model-answer PDFs, and model answers for the Kannada language papers may have more KN-Nandi text. Documents in either font can be proposed through a GitHub issue.

## Newspaper fonts on archived pages

Kannada newspapers set their web pages in legacy fonts from 1999 to about 2009 and served the fonts to readers. On 17 November 2026 I searched the Internet Archive for 4 of these fonts, used by 3 newspapers, and found 4 more on the same sites. Every page, font file and font page found is in [newspaper-fonts.tsv](newspaper-fonts.tsv), with its capture date and archive URL. No font file was downloaded.

To work out each font's code page, I read the Kannada text on the saved pages with each of the 5 tables. A font whose text reads with no unknown glyphs and no malformed syllables uses that table's code page.

| Font | Newspaper and years | Text read | Code page | Font served with the pages |
|---|---|---|---|---|
| LangscapeKndPadmini | Samyukta Karnataka, 1999 to 2000 | 43 runs, 2,190 codes | Akruti: no problems | `lskpad-n.ttf`, 2001 |
| LLPad | Samyukta Karnataka, 2000 | 3 runs, 23 codes | Akruti: no problems | None found |
| SKPadmini | Samyukta Karnataka, 2001 | 17 runs, 10,536 codes | Akruti: 1 unknown glyph (FC) and 1 malformed syllable, in 1 run with a doubled ು | `skpan___.ttf`, 2001 |
| KN-TTNandi | Kannada Prabha, 2000 to 2005 | 13 runs, 169 codes | None of the 5 fits, on little text | PFR 2002, EOT 2006 |
| KNW-TTNandi | Kannada Prabha, 2002 to 2008 | 247 runs, 15,146 codes | None of the 5 | PFR and EOT, 2004 |
| SHREE_DECCAN | Prajavani, 2003 to 2006 | 217 runs, 56,564 codes | None of the 5, Shree-Lipi included | PFR, 2004 |
| Shree_DecMod | Prajavani, 2008 | 10 runs, 11,156 codes | None of the 5 | EOT, 2004 to 2006 |
| LangscapeKndPadma | Prajavani, 2000 to 2001 | 27 runs, 5,677 codes | None of the 5 | `kndafont.zip`, 2004 and 2005 |

A run is the text of 1 font tag. So:

- **LangscapeKndPadmini and SKPadmini use the Akruti code page**, and so, on only 23 codes, does LLPad. The tool converts them today with `--legacy-font "LangscapeKndPadmini=akruti"` (and the same for the others). They are not yet in the built-in font list.
- **KNW-TTNandi, SHREE_DECCAN, Shree_DecMod and LangscapeKndPadma are layouts of their own**, and KN-TTNandi probably is: none of the 5 tables fits it, but only 169 of its codes were read. Despite its name, SHREE_DECCAN is not Shree-Kan-0850's layout. KNW-TTNandi and SHREE_DECCAN have 247 and 217 runs of public text; their lines are not counted yet against the 100-line bar. A table for either one needs its font, which the archive holds as the PFR or EOT the newspaper served. PFR and EOT are web font formats that need converting before the font can be rendered.
- **Which font file Prajavani's SHREE_DECCAN pages used is unclear.** The home page of June 2004 loads `akpadmi.pfr`, and `SHREE_DECCAN.pfr` was first captured in March 2004. `IncludeEOT.js` gives SHREE_DECMOD the file `SHREEDE0.eot`, captured since June 2004. Whether SHREE_DECCAN and Shree_DecMod share a code page is not known.

Newspaper text is not kept in this repository.
