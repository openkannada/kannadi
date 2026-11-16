# Shree-Lipi and Akruti: sources and method

Status: checked on 16 November 2026. Akruti is built: see [akruti.md](akruti.md). No converter for Shree-Lipi exists yet. Nothing here is legal advice; whether a layout table derived as described below may carry rights in India has not been researched and is unresolved.

## Summary

- Shree-Lipi (Modular Infotech; fonts named `Shree-Kan-0850` and similar) is a widely used Kannada layout besides Nudi and Baraha. No public code page for it was found.
- A Shree-Lipi table could be derived from public PDFs that use Shree-Lipi fonts, without reading the fonts themselves. No PDF with a real Shree-Lipi font name was found yet among the Karnataka sources checked.
- Akruti (`AkrutiKndPadma`) appears with its real font name in public Karnataka PDFs. It is a separate layout from Nudi and needs its own table. Its sources are confirmed, so it can start before Shree-Lipi.
- Asking Modular Infotech to publish the Shree-Lipi code page under an open license would avoid derivation. That is a decision for the maintainer.

## Rules for this work

- Do not download Shree-Lipi or Akruti font files from any source other than the vendor.
- Fonts embedded in a public document may be rendered, and their glyph names and encodings read, to identify which letter each code draws (the maintainer's decision, 16 November 2026). The fonts, their outlines and their files are never committed or redistributed; a table records only the code and the letter.
- Do not copy tables from other converters (see [prior-art.md](prior-art.md)).
- Keep only short excerpts of source documents, with their URLs, as for the Nudi fixtures.

For background: typeface designs are generally not protected by copyright in the US, while font software can be, and US design patents can cover letterforms ([overview](https://en.wikipedia.org/wiki/Intellectual_property_protection_of_typefaces), [Fordham IPLJ](https://fordhamiplj.org/2021/12/11/open-type-questions-issues-in-copyright-law-and-protecting-typeface-design/)). Product names such as Shree-Lipi are trademarks and are used here only to name the layout.

## What was found

### Shree-Lipi

- The font name `Shree-Kan-0850` and the file `SHREE850.ttf` are named as a non-Unicode Kannada encoding in a [Mozilla bug report](https://bugzilla.mozilla.org/show_bug.cgi?id=213213).
- [Baraha's converter](https://baraha.com/help/FontConvert.htm) lists Shree-Kan-0850 among the fonts it converts. It is closed source and states no license for its tables.
- The [Modular Infotech site](https://www.modular-infotech.com) lists a blog post titled "Convert Shree-Lipi to Unicode Online" and a Downloads page. Their content and the site's terms could not be read. No code page was found there.
- 2 Karnataka Legislative Assembly member pages, [13mem/50.pdf](http://web.archive.org/web/20150427105120/http://kla.kar.nic.in/13mem/50.pdf) and [13mem/75.pdf](http://web.archive.org/web/20160304030357/http://kla.kar.nic.in/13mem/75.pdf), embed a TrueType subset whose name was replaced with `TTE10AECA0t00` when the PDF was made with PScript5 and Ghostscript 8.15. The extracted text does not decode with the Nudi table: 50.pdf gives 145 unknown glyphs and 267 malformed syllables. The layout is unknown; it could be Shree-Lipi.

### Akruti

Karnataka Acts in Kannada on `dpal.kar.nic.in`, made with PageMaker 6.52 and Acrobat Distiller 5.0, embed `AkrutiKndPadmaBold` and `AkrutiKndPadmaDemi` as Type 1C fonts with a custom encoding and no ToUnicode map. Examples, all fetched from the Internet Archive:

- [Roerich and Devikarani Roerich Act, 19 of 2001](http://dpal.kar.nic.in/ao2001/(19%20of%202001)%20Roerich%20&%20Devikarani%20Roerich%20Act-(K).pdf)
- [Reservation Act, 1 of 2001](http://dpal.kar.nic.in/ao2001/(1%20of%202001)%20Reservation%20Act-(K).pdf)
- [Hindu Religious Institutions Act, 33 of 2001](http://dpal.kar.nic.in/ao2001/(33%20of%202001)%20Hindu%20Religious%20Institutions%20Act-(K).pdf)
- [Appropriation Act, 2 of 2002](http://dpal.kar.nic.in/ao2002/(%202%20of%202002)%20Appropriation%20Act%20-%20(K).pdf)

About 19 more Kannada Acts are listed in the [Internet Archive index for dpal.kar.nic.in](http://web.archive.org/cdx/search/cdx?url=dpal.kar.nic.in&matchType=domain&filter=mimetype:application/pdf&collapse=urlkey). The 2002 budget PDFs [bh2002k.pdf](http://web.archive.org/web/20040822015443/http://www.kar.nic.in:80/finance/bud2002-kan/bh2002k.pdf) and [bs2002-k.pdf](http://web.archive.org/web/20060517033426/http://kar.nic.in:80/finance/bud2002-kan/bs2002-k.pdf) also use `AkrutiKndPadma`.

Akruti text does not decode with the Nudi table. On page 1 of `bh2002k.pdf`, the Nudi converter reports 68 unknown glyphs and 212 malformed syllables. That is 1 page; it shows the layouts differ, not by how much. Baraha lists a font named `AkrutiKndPadmini`. Whether all Akruti Kannada fonts share 1 layout is not known, so each font name may need checking.

### Other public Kannada PDFs checked

| Document | Fonts | Layout |
|---|---|---|
| Karnataka Gazette Part V, 10 February 2025 ([PDF](http://web.archive.org/web/20250421085341/https://karnatakajudiciary.kar.nic.in/newwebsite/common_folder/notification/8187.pdf)) | CID TrueType, Identity-H, names stripped | Nudi ANSI codes in the extracted text, next to Unicode text that extracts with broken letter order |
| Recruitment notice for stenographers, 2015 ([PDF](http://web.archive.org/web/20150922093911/http://www.karnataka.gov.in:80/2015/RECRUITMENT_OR_STENO_and_TYPIST.pdf)) | Nudi 04 k, both WinAnsi and Identity-H | Nudi |
| Unicode L2/03-068 from Karnataka DIT ([PDF](https://www.unicode.org/L2/L2003/03068-kannada.pdf)) | NudiAkshara, WinAnsi | Nudi |
| 2002 budget summary ([BGLAN2002.pdf](http://web.archive.org/web/20090410021533/http://www.kar.nic.in/finance/bud2002-kan/BGLAN2002.pdf)) | BRH Kannada, WinAnsi | Baraha ANSI, same as Nudi |

These are test inputs for PDF text extraction.

## How a table is derived

The Akruti table was built this way, and Shree-Lipi will follow it. [akruti.md](akruti.md) has the details.

1. Collect public PDFs whose font names, as `pdffonts` lists them, identify the layout.
2. Render each glyph of the embedded fonts and identify it, with glyphs shaped by HarfBuzz from Unicode Kannada fonts as a second opinion.
3. Convert every document, and read each unknown glyph and malformed syllable against the rendered page.
4. Store the result as a table in the format of `data/nudi.tsv`, with its sources in the header, and add the layout to `Layout`.
5. Build a golden fixture from short excerpts with source URLs, read against the page, and report accuracy per document and per font.

A table covers the codes that appear in the collected documents. Rare conjunct glyphs may stay unknown; the converter reports them.

A table is ready when every code seen in the collected documents has a checked entry or is marked unknown, and the golden fixture has at least 100 lines from 3 or more documents that convert with no differences. A Kannada reader's check of the fixture is still to be done for Akruti.

## Decisions for the maintainer

1. Whether to ask Modular Infotech to publish the Shree-Lipi code page under an open license, such as CC0 for the data. The research found `support@modular-infotech.com` as a contact; it was not contacted. Modular offers its own conversion tool, so it may decline.
2. A Kannada reader's check of the 125 Akruti fixture lines.
3. Where to look for Shree-Lipi documents with real font names. Candidates are publishers' and newspapers' PDF archives from before 2010, and university question papers.
