# Word 97 to 2003 and RTF files

Status: `kannadi to-unicode` reads `.doc` files written by Word 97 and later, and `.rtf` files, and writes their text to `NAME.doc.txt` and `NAME.rtf.txt`, with runs in legacy fonts converted to Unicode. This note says how it reads them, what was measured, and what does not work.

## How legacy text sits in a Word file

A `.doc` file is an OLE compound file. Its WordDocument stream holds the text and the property pages; the 0Table or 1Table stream holds the piece table, the stylesheet and the font table ([MS-DOC]). Text typed in a Nudi or Baraha font is stored as the font's glyph codes: 1 byte each in an 8-bit piece, which Word reads as Windows-1252, or the Windows-1252 character of the code in a UTF-16 piece. Word stores the codes of a symbol font, such as some Baraha installs, at U+F020 to U+F0FF. RTF writes the same codes as `\'hh` bytes, or as `\uN` with a `\'hh` fallback.

So the legacy text of a run is its characters as Windows-1252 codes, and the run's font gives the layout. The work is finding each character's font.

## What the tool does

- **Text.** The piece table gives each character position's offset in the WordDocument stream, and whether it is 8-bit or UTF-16. Field instructions (between U+0013 and U+0014) and tracked deletions (sprmCFRMarkDel) are left out; field results and hidden text are kept. A symbol inserted with Insert Symbol (sprmCSymbol), which the text holds as a placeholder, is read as the symbol's character in its own font. Paragraph marks, cell marks, line breaks and page breaks end a line; the mark that ends a table row, which the paragraph properties flag with sprmPFTtp, adds none. Pictures, note references and optional hyphens are dropped.
- **Fonts.** A character's font comes from the stylesheet defaults, then its paragraph style and the styles it is based on, its character style, its run's own properties and its piece's own properties, each over the last. A character below U+0080 is drawn in the ASCII font (sprmCRgFtc0), and others in the high-ANSI font (sprmCRgFtc2). The font number indexes the font table.
- **Stories.** Footnotes, headers and footers, comments, endnotes and text boxes follow the body, each after a blank line. Headers and footers are split by PlcfHdd, and one that several sections repeat is written once.
- **RTF.** The font table names each `\fN`. `\plain` returns to the `\deff` font. `\uN` is read as Unicode and its `\ucN` fallback skipped; `\'hh` and 8-bit text are read as Windows-1252. Destinations with no document text are skipped (the stylesheet, pictures, `\bin` data, field instructions, unknown `\*` destinations), and of a `\upr` group only the Unicode copy in `\*\ud` is read. Tracked deletions (`\deleted`) and list numbers (`\listtext`, `\pntext`) are left out, as a `.doc` file holds no list numbers as text. A surrogate pair written as 2 `\uN` values gives 1 character. Footnotes, headers and footers, comments and `\shptxt` text boxes become their own parts of the output.
- **Conversion.** Runs in the same layout are joined, so a word Word split across runs converts whole. A character with no Windows-1252 code inside a legacy run, such as Unicode Kannada typed while a Nudi font was chosen, is kept as it is. Fonts named as legacy with no table, such as Shree-Kan-0851, are reported as `not converted` and their text left out.
- **Limits.** Streams are read up to 64 MB, and a document may hold at most 2^26 characters and no more characters than its text stream has bytes. Style chains stop after 11 levels and RTF groups after 512. Work grows in step with the file, so a hostile file cannot hang the tool; memory does too, at up to about 100 times the text for a file made of very short paragraphs. Word 6 and Word 95 files and password-protected files are refused with a reason.

## What was measured

Fixtures: `crates/kannadi-cli/tests/fixtures/legacy.docx` holds lines of the Nudi and Akruti fixtures in Nudi 01 e, BRH Kannada, Nudi 04 k and AkrutiKndPadma, a paragraph style, English and Unicode runs, a table and a header. LibreOffice 26.8.3.1 saved it as `.doc` and `.rtf`; both read back exactly as `legacy.expected.txt`.

Real documents, downloaded on 22 November 2026. The Karnataka government hosts (`*.karnataka.gov.in`, `*.kar.nic.in`) time out from outside India, the Internet Archive was offline and the Common Crawl index returned errors, so the sample comes from hosts that answered. A search for the Nudi codes of ಮತ್ತು in Word files found Nudi `.doc` files on the Labour Commissionerate, Electrical Inspectorate and other Karnataka government hosts, which could not be reached, and 3 Karnataka school exam papers on a teachers' forum, which could. A crawl of 31 district portals (1,102 pages) found no `.doc` or `.rtf` links. A crawl of state university sites found 7, of which 6 downloaded.

| Source | Files | Legacy text | Result |
|---|---|---|---|
| Bagalkot district SSLC preparatory exam 2015 (mathematics), and 8th and 9th standard unit tests, shared on the [mathssciencestf](https://groups.google.com/g/mathssciencestf) Google Group | 3 | Nudi, with English and mathematics in other fonts | 345 lines, 7,361 Kannada characters. 4 unknown glyphs and 1 malformed syllable, each in the source as typed: a lone Nudi vowel sign after a Latin letter (3), a stray `s` (1), and a missing glyph in ಚೆಂಡುಗಳು (1) |
| Rani Channamma University, Belagavi: 2027-28 bond paper form | 1 | None: Unicode Kannada | 236 words, 1,818 Kannada characters, all kept; the same words as LibreOffice's text export |
| Mangalore University: 5 syllabus files | 5 | None: English | Read with no errors. The words match LibreOffice's text export at 96.0% to 100%; the difference is list numbers, which Word draws and does not store, and text boxes, which LibreOffice's text export leaves out |

The downloaded files are not kept in the repository.

## Not done

- `\'hh` text in non-legacy RTF fonts in a code page other than Windows-1252 is read as Windows-1252.
- Word 6 and Word 95 files, whose FIB and text layout differ.
- List numbers and bullets, which Word draws and does not store as text.
- Paragraph properties stored outside the property page (sprmPHugePapx), as in very wide tables: their row ends are not seen, so each such row adds a blank line.
- Output is text. A DOCX written from a `.doc` would keep paragraphs and runs, but needs its own change.
