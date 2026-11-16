# Shree-Lipi and Akruti: sources and method

Status: checked on 17 November 2026. Akruti is built: see [akruti.md](akruti.md). Shree-Lipi is built for Shree-Kan-0850: see [The Shree-Lipi table](#the-shree-lipi-table). The rules below say how layout tables are derived. They record the maintainer's decisions of 16 and 17 November 2026 and are not legal advice.

## Summary

- Shree-Lipi (Modular Infotech; fonts named `Shree-Kan-0850` and similar) is a widely used Kannada layout besides Nudi and Baraha. No public code page for it was found.
- No PDF with a real Shree-Lipi font name was found among the Karnataka sources checked, including the 98 archived Kannada Acts on `dpal.kar.nic.in` (Akruti, Nudi and BRH fonts only). The Udayavani newspaper's web pages of 2003 and 2004 set their text in `Shree-Kan-0850` and served the font to readers. The Shree-Lipi table was derived from that font and checked on those pages. See [Searching for Shree-Lipi sources](#searching-for-shree-lipi-sources).
- Akruti (`AkrutiKndPadma`) appears with its real font name in public Karnataka PDFs. It is a separate layout from Nudi and needs its own table. Its sources are confirmed, so it can start before Shree-Lipi.
- Asking Modular Infotech to publish the Shree-Lipi code page under an open license would avoid derivation. That is a decision for the maintainer.

## Rules for deriving a layout table

- A table maps each glyph code to the Kannada it draws. The maintainer treats that mapping as factual data, so it may be derived by rendering a font, whether the font is installed or embedded in a public document, and by reading the font's glyph names and encodings.
- Font programs, and glyph outlines extracted from them, are never committed or redistributed. Only the table is.
- Each table's header records how it was derived: the fonts and documents, and the method. See `crates/kannadi/data/nudi.tsv` and `crates/kannadi/data/akruti.tsv`.
- Do not download Shree-Lipi or Akruti font files from any source other than the vendor, or a publisher that served the font to readers with its public pages, as Udayavani did (decided on 17 November 2026).
- Do not copy tables from other converters (see [prior-art.md](prior-art.md)).
- Keep only short excerpts of source documents, with their URLs, as for the Nudi fixtures.

These rules are about building tables. At run time, the `kannadi` tool reads the ToUnicode maps, `Differences` lists and other font data in the user's own file to convert it; that is outside these rules.

For background: typeface designs are generally not protected by copyright in the US, while font software can be, and US design patents can cover letterforms ([overview](https://en.wikipedia.org/wiki/Intellectual_property_protection_of_typefaces), [Fordham IPLJ](https://fordhamiplj.org/2021/12/11/open-type-questions-issues-in-copyright-law-and-protecting-typeface-design/)). Product names such as Shree-Lipi are trademarks and are used here only to name the layout.

## What was found

### Shree-Lipi

- The font name `Shree-Kan-0850` and the file `SHREE850.ttf` are named as a non-Unicode Kannada encoding in a [Mozilla bug report](https://bugzilla.mozilla.org/show_bug.cgi?id=213213).
- [Baraha's converter](https://baraha.com/help/FontConvert.htm) lists Shree-Kan-0850 among the fonts it converts. It is closed source and states no license for its tables.
- The [Modular Infotech site](https://www.modular-infotech.com) lists a blog post titled "Convert Shree-Lipi to Unicode Online" and a Downloads page. Their content and the site's terms could not be read. No code page was found there.
- 2 Karnataka Legislative Assembly member pages, [13mem/50.pdf](http://web.archive.org/web/20150427105120/http://kla.kar.nic.in/13mem/50.pdf) and [13mem/75.pdf](http://web.archive.org/web/20160304030357/http://kla.kar.nic.in/13mem/75.pdf), embed a TrueType subset whose name was replaced with `TTE10AECA0t00` when the PDF was made with PScript5 and Ghostscript 8.15. The extracted text does not decode with the Nudi table: 50.pdf gives 145 unknown glyphs and 267 malformed syllables. Rendering the subset shows a third layout: Latin letters and digits at their ASCII codes, and Kannada glyphs from 0x81 up. Its family name is gone. Its glyphs differ from Shree-Kan-0850's at every code, Latin letters and digits included, so it is not that font.

### Akruti

Karnataka Acts in Kannada on `dpal.kar.nic.in`, made with PageMaker 6.52 and Acrobat Distiller 5.0, embed `AkrutiKndPadmaBold` and `AkrutiKndPadmaDemi` as Type 1C fonts with a custom encoding and no ToUnicode map. Examples, all fetched from the Internet Archive:

- [Roerich and Devikarani Roerich Act, 19 of 2001](http://dpal.kar.nic.in/ao2001/(19%20of%202001)%20Roerich%20&%20Devikarani%20Roerich%20Act-(K).pdf)
- [Reservation Act, 1 of 2001](http://dpal.kar.nic.in/ao2001/(1%20of%202001)%20Reservation%20Act-(K).pdf)
- [Hindu Religious Institutions Act, 33 of 2001](http://dpal.kar.nic.in/ao2001/(33%20of%202001)%20Hindu%20Religious%20Institutions%20Act-(K).pdf)
- [Appropriation Act, 2 of 2002](http://dpal.kar.nic.in/ao2002/(%202%20of%202002)%20Appropriation%20Act%20-%20(K).pdf)

About 19 more Kannada Acts are listed in the [Internet Archive index for dpal.kar.nic.in](http://web.archive.org/cdx/search/cdx?url=dpal.kar.nic.in&matchType=domain&filter=mimetype:application/pdf&collapse=urlkey). The 2002 budget PDFs [bh2002k.pdf](http://web.archive.org/web/20040822015443/http://www.kar.nic.in:80/finance/bud2002-kan/bh2002k.pdf) and [bs2002-k.pdf](http://web.archive.org/web/20060517033426/http://kar.nic.in:80/finance/bud2002-kan/bs2002-k.pdf) also use `AkrutiKndPadma`.

Akruti text does not decode with the Nudi table. On page 1 of `bh2002k.pdf`, the Nudi converter reports 68 unknown glyphs and 212 malformed syllables. That is 1 page. It shows that the layouts differ; it does not show by how much. Baraha lists a font named `AkrutiKndPadmini`. Whether all Akruti Kannada fonts share 1 layout is not known, so each font name may need checking.

### Other public Kannada PDFs checked

| Document | Fonts | Layout |
|---|---|---|
| Karnataka Gazette Part V, 10 February 2025 ([PDF](http://web.archive.org/web/20250421085341/https://karnatakajudiciary.kar.nic.in/newwebsite/common_folder/notification/8187.pdf)) | CID TrueType, Identity-H, names stripped | Nudi ANSI codes in the extracted text, next to Unicode text that extracts with broken letter order |
| Recruitment notice for stenographers, 2015 ([PDF](http://web.archive.org/web/20150922093911/http://www.karnataka.gov.in:80/2015/RECRUITMENT_OR_STENO_and_TYPIST.pdf)) | Nudi 04 k, both WinAnsi and Identity-H | Nudi |
| Unicode L2/03-068 from Karnataka DIT ([PDF](https://www.unicode.org/L2/L2003/03068-kannada.pdf)) | NudiAkshara, WinAnsi | Nudi |
| 2002 budget summary ([BGLAN2002.pdf](http://web.archive.org/web/20090410021533/http://www.kar.nic.in/finance/bud2002-kan/BGLAN2002.pdf)) | BRH Kannada, WinAnsi | Baraha ANSI, same as Nudi |

These are test inputs for PDF text extraction.

## Searching for Shree-Lipi sources

Checked on 16 November 2026, with the Akruti work.

- **Karnataka Acts.** All 98 archived Kannada Acts and Ordinances on `dpal.kar.nic.in` were downloaded and their fonts listed. Those of 2001 to 2003 use AkrutiKndPadma; later ones use Nudi 01 e, Nudi 03 e, Nudi Akshara, NudiB-Akshar and BRH Kannada. 1 Act of 2003 also uses AkliteKndPadmini, a layout of its own. None uses a Shree-Lipi font.
- **Udayavani, 2003.** The newspaper's archived home page (`199.74.239.15/udayavani/home.asp`, Internet Archive, April 2003) sets its Kannada in `<font face="Shree-Kan-0850">`, with text such as `ÃÜã±ÜñÝÃÝ`. The site offered the font to readers as `SHREE850.ttf` and as the embedded web font `SHREEKA0.eot`. These pages are Shree-Lipi text with a known font name, so they can check a table. They carry no glyph images, so they cannot build one alone.
- **Legislative Assembly member pages.** The unnamed font in `13mem/50.pdf` is a third layout, described above. Its subset holds only the 117 glyphs that page uses, too few to test the Udayavani text against, so whether it is Shree-Kan-0850 is not known.
- **Web search.** Searches for PDFs naming `Shree-Kan` fonts found only font shops and converters.

The maintainer decided on 17 November 2026 that the font Udayavani served with its pages may be rendered to build the table, with the pages to check it. The table below was built that way.

## The Shree-Lipi table

`Layout::ShreeLipi` converts text in Shree-Kan-0850 in both directions. The table is [shree-lipi.tsv](../crates/kannadi/data/shree-lipi.tsv). Font names follow `legacy::font_layout`: Shree-Kan-0850 and its weights get this layout, and other Shree-Kan numbers are still known as legacy fonts with no table, because their layouts were not checked.

### The layout

Shree-Lipi stores Kannada the way Nudi and Akruti do, at its own codes: a consonant shape, then the head stroke (DC) or a vowel sign, then subscripts, then the arkavattu (EC). ಾ is DD, ು is E1, ೂ is E3, ೆ is E6, the length mark ೕ is E0, ೖ is E7, ೃ is EA, ಂ is ED and the virama is 85. ಪ, ಫ and ವ take their own ು (E2) and ೂ (E4). Letters with ಿ have glyphs of their own, such as ಕಿ at 51.

ಮ is ವ with the head stroke and a tail (CA DC E1); ಮಾ uses the tail with ಾ (DE), and ಮ್ the tail before the virama (E5). ಯ is a base of its own with the same tails (BF E1). ಝ is ರ with a hook (6B) and a tail. ಋ is 4D followed by the ು tail. Some glyphs draw 2 subscripts, such as ್ಟ್ರ (F3) and ್ತ್ರ (F7), or a subscript with its sign, such as ್ಕೃ (A2) and ್ತೈ (F4). Latin digits stay at the ASCII codes. The font draws ‘ at 22, the hyphen at 26 and a short bar at 3E, used after abbreviations and numbers (ಸಂ।ರದ); the table reads 3E as ।. It draws nothing at 2D.

Typing habits the table reads: the wide blank 24 typed before a subscript or ೖ to move it right; ಾ with ೖ for ೈ in loanwords such as ಶಾಂಘೈ and ಸೊಸೈಟಿ; and ಕ್ಷ and ಜ್ಞ as glyphs of their own (FB, FE), which the encoder writes as ಕ and ಜ with a subscript, the way most of the text has them.

### Source and method

The font is Shree-Kan-0850 version 1.0 of 18 March 2000, as the Udayavani newspaper served it to readers at `199.74.239.15/udayavani/fonts/SHREE850.ttf`, fetched from the Internet Archive ([capture of 10 April 2003](http://web.archive.org/web/20030410020653/http://199.74.239.15:80/udayavani/fonts/SHREE850.ttf)). The font file is not in this repository.

1. Render every glyph with its advance width and read it, with glyphs shaped by HarfBuzz from Noto Sans Kannada as a second opinion.
2. Extract the Kannada text of 173 archived Udayavani story pages captured in 2003 and 2004, which name Shree-Kan-0850 in their font tags.
3. List the words each code appears in, and settle each code from the words: ಘೋಷಿತ shows that 4E is ಘ with ೆ, ಹರಿಣಾಕ್ಷಿ that FC is ಕ್ಷಿ.
4. Convert all the text, and read every unknown glyph and malformed syllable against the same codes rendered in the font.

Codes the font has but no page used are left out, and are reported as unknown if they appear.

### Results

| Measure | Result |
|---|---|
| Pages | 173 Udayavani story pages, captured in 2003 and 2004 |
| Text | 1,660 distinct lines, 233,346 glyphs |
| Unknown glyphs | 0 |
| Malformed syllables | 23 |
| Fixture lines read against the rendered font | 130 lines from 36 pages, all equal |
| Words that encode back to the bytes typed | 28,015 of 28,783 (97.3%); in the fixture, 765 of 787 |
| Fixture words read alone that `best_layout` gives another layout | 11 of 787 |

Of the 23 malformed syllables, 12 are ಆ್ಯ, the way these pages write the vowel of English "and" (ಆ್ಯಂಡ್, ಆ್ಯಂಟನಿ); the converter keeps it and reports it, as Unicode has no valid syllable for it. The rest are the pages' own typing errors: ೖ on ಮೊ and ನೊ for ಮೊಯ್ಲಿ and ನೊಯ್ಡಾ, a subscript typed twice, a stray ಾ, ರು with ೖ, and a consonant with no head stroke.

The fixture is [shree-lipi.tsv](../crates/kannadi/tests/fixtures/shree-lipi.tsv), with the source URL of each line. The pages have no page images, so each line was checked against its codes rendered in Shree-Kan-0850.

### Limits

- Only Shree-Kan-0850 was checked. Other Shree-Kan fonts may share the layout, but they are left out until a document confirms it.
- Udayavani's story pages name Shree-Kan-0850 only in some `<font face>` tags. The story text takes its font from an external style sheet, which the `kannadi` tool does not fetch, so that text is left as it is. Save it as a text file and use `--layout shree-lipi`.
- Typists stored some clusters in more than 1 way, such as ್ಟ್ರ as 1 glyph or 2, and ನ್ನ as a half form or a subscript. Both read the same; the encoder writes 1 of them.

## How a table is derived

The Akruti and Shree-Lipi tables were built this way. [akruti.md](akruti.md) has the details for Akruti.

1. Collect public PDFs whose font names, as `pdffonts` lists them, identify the layout.
2. Render each glyph of the embedded fonts and identify it, with glyphs shaped by HarfBuzz from Unicode Kannada fonts as a second opinion.
3. Convert every document, and read each unknown glyph and malformed syllable against the rendered page.
4. Store the result as a table in the format of `crates/kannadi/data/nudi.tsv`, with its sources in the header, and add the layout to `Layout`.
5. Build a golden fixture from short excerpts with source URLs, read against the page, and report accuracy per document and per font.

A table covers the codes that appear in the collected documents. Rare conjunct glyphs may stay unknown; the converter reports them.

A table is ready when every code seen in the collected documents has a checked entry or is marked unknown, and the golden fixture has at least 100 lines from 3 or more documents that convert with no differences. A Kannada reader's check of the fixture is still to be done for Akruti and Shree-Lipi.

## Decisions for the maintainer

1. Whether to ask Modular Infotech to publish the Shree-Lipi code page under an open license, such as CC0 for the data. The research found `support@modular-infotech.com` as a contact; it was not contacted. Modular offers its own conversion tool, so it may decline.
2. A Kannada reader's check of the 125 Akruti and 130 Shree-Lipi fixture lines.
3. Where to look for Shree-Lipi documents with real font names. Candidates are publishers' and newspapers' PDF archives from before 2010, and university question papers.
