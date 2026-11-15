# Shree-Lipi and Akruti: what can be built legally

Status: research only, checked on 16 November 2026. No converter for these layouts exists in this crate yet.

## Summary

- Shree-Lipi (Modular Infotech, fonts named `Shree-Kan-0850` and similar) is the most common Kannada layout after Nudi and Baraha. Its fonts are commercial, and its code page is not published.
- A Shree-Lipi table can be derived legally from public PDFs that embed Shree-Lipi font subsets, by reading which glyph code draws which letter. No such PDF with a real Shree-Lipi font name was found yet among the Karnataka sources checked.
- Akruti (Cyberscape) is confirmed in public Karnataka PDFs with its real font name. It is a separate layout from Nudi, so it needs its own table. It is the more practical next layout.
- The cleanest route for Shree-Lipi is to ask Modular Infotech to publish the code page under a permissive license. That decision is Prasad's.

## Rules for this work

- Do not download Shree-Lipi or Akruti font files from any site. Copies offered for free download are not licensed.
- Do not extract or redistribute the font programs embedded in PDFs.
- Do not copy tables from other converters (see [prior-art.md](prior-art.md)).
- A table here records facts: glyph code N in layout L draws letter X. It is derived from documents a reader can open, and each entry records its source.

This is a working position, not legal advice. Typeface designs are generally not protected by copyright in the US, while font software can be ([overview](https://en.wikipedia.org/wiki/Intellectual_property_protection_of_typefaces), [Fordham IPLJ](https://fordhamiplj.org/2021/12/11/open-type-questions-issues-in-copyright-law-and-protecting-typeface-design/)). Indian law was not researched. Product names such as Shree-Lipi are trademarks and are used here only to name the layout.

## What was found

### Shree-Lipi

- The font name `Shree-Kan-0850` and the file `SHREE850.ttf` are confirmed as a non-Unicode Kannada encoding in a [Mozilla bug report](https://bugzilla.mozilla.org/show_bug.cgi?id=213213).
- [Baraha's converter](https://baraha.com/help/FontConvert.htm) lists Shree-Kan-0850 among the fonts it converts. It is closed source and states no license for its tables.
- No public code page from Modular Infotech was found on [modular-infotech.com](https://www.modular-infotech.com).
- 2 Karnataka Legislative Assembly member pages, [13mem/50.pdf](http://web.archive.org/web/20150427105120/http://kla.kar.nic.in/13mem/50.pdf) and [13mem/75.pdf](http://web.archive.org/web/20160304030357/http://kla.kar.nic.in/13mem/75.pdf), use an embedded TrueType subset whose name Ghostscript replaced with `TTE10AECA0t00`. The text uses high Latin-1 codes in a layout that is neither Nudi nor Akruti. It may be Shree-Lipi; this is not confirmed.

### Akruti

Karnataka Acts in Kannada on `dpal.kar.nic.in`, made with PageMaker 6.52 and Acrobat Distiller 5.0, embed `AkrutiKndPadmaBold` and `AkrutiKndPadmaDemi` as Type 1C fonts with a custom encoding and no ToUnicode map. Examples, from the Internet Archive:

- [Roerich and Devikarani Roerich Act, 19 of 2001](http://dpal.kar.nic.in/ao2001/(19%20of%202001)%20Roerich%20&%20Devikarani%20Roerich%20Act-(K).pdf)
- [Reservation Act, 1 of 2001](http://dpal.kar.nic.in/ao2001/(1%20of%202001)%20Reservation%20Act-(K).pdf)
- [Hindu Religious Institutions Act, 33 of 2001](http://dpal.kar.nic.in/ao2001/(33%20of%202001)%20Hindu%20Religious%20Institutions%20Act-(K).pdf)
- [Appropriation Act, 2 of 2002](http://dpal.kar.nic.in/ao2002/(%202%20of%202002)%20Appropriation%20Act%20-%20(K).pdf)

About 19 more Kannada Acts are listed in the [Internet Archive index for dpal.kar.nic.in](http://web.archive.org/cdx/search/cdx?url=dpal.kar.nic.in&matchType=domain&filter=mimetype:application/pdf&collapse=urlkey). The 2002 budget PDFs under `kar.nic.in/finance/bud2002-kan/` also embed `AkrutiKndPadma`. Extracted text such as `pê³©îdÒ` does not decode with the Nudi table: on page 1 of `bh2002k.pdf`, the Nudi converter reports 68 unknown glyphs and 212 malformed syllables.

### Other public Kannada PDFs checked

| Document | Fonts | Layout |
|---|---|---|
| Karnataka Gazette Part V, 10 February 2025 ([PDF](http://web.archive.org/web/20250421085341/https://karnatakajudiciary.kar.nic.in/newwebsite/common_folder/notification/8187.pdf)) | CID TrueType, Identity-H, names stripped | Nudi ANSI codes through ToUnicode, mixed with Unicode Nudi text |
| Recruitment notice for stenographers, 2015 ([PDF](http://web.archive.org/web/20150922093911/http://www.karnataka.gov.in:80/2015/RECRUITMENT_OR_STENO_and_TYPIST.pdf)) | Nudi 04 k, Identity-H | Nudi |
| Unicode L2/03-068 from Karnataka DIT ([PDF](https://www.unicode.org/L2/L2003/03068-kannada.pdf)) | NudiAkshara, WinAnsi | Nudi |
| 2002 budget summary, `bud2002-kan/BGLAN2002.pdf` | BRH Kannada, WinAnsi | Baraha ANSI, same as Nudi |

These are inputs for the PDF work, not for Shree-Lipi.

## How a table would be derived

This is the same method used for `nudi.tsv`, applied to documents:

1. Collect public PDFs whose embedded font names identify the layout. `pdffonts` lists them.
2. Pick documents that also exist as Unicode text, or that a Kannada reader can read from the rendered page.
3. Render each page as an image, which any PDF reader does when showing it. Do not extract the font program.
4. For each glyph code used, record the letter it draws, with the document and page it was read from. A Kannada reader checks every entry.
5. Store the result as a new table next to `data/nudi.tsv`, in the same format. The decoder and encoder are already generic over the table; a second layout needs a font parameter on the public API.
6. Build a golden fixture from short excerpts, with source URLs, and report accuracy as for Nudi.

A layout table derived this way covers the codes that appear in the collected documents. Rare conjunct glyphs may stay unknown, and the converter reports them.

## Decisions for Prasad

1. Whether to ask Modular Infotech for a published Shree-Lipi code page under a permissive license. This is the fastest legal route and also the most complete.
2. Whether Akruti should come first. Its sources are confirmed and public. Shree-Lipi needs documents with real font names first.
3. Where to look for Shree-Lipi documents. Likely sources are publishers' and newspapers' PDF archives from before 2010, and university question papers. A reader with access to such archives would speed this up.
