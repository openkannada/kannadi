# Text from PDFs typeset in legacy fonts

Status: `kannadi to-unicode` reads PDF files and writes their text as `NAME.pdf.txt`, with text in Nudi, Baraha ANSI and Akruti fonts converted to Unicode. This note says how it works, what was measured, and what does not work.

## How legacy text sits in a PDF

A PDF shows text with operators such as `Tj` and `TJ`, which give a string of codes in the current font. For a legacy font, those codes are the same glyph codes the font uses in a Word file or a web page, so they can go straight to `legacy::to_unicode` with the font's layout. The work is in finding the codes and the fonts:

1. **1-byte fonts with WinAnsi encoding.** Word and Distiller output from the 2000s embeds BRH and Nudi fonts this way, often as subsets named like `CCMNNO+BRHKannada`. The string bytes are the glyph codes.
2. **2-byte CID fonts with a ToUnicode map.** Newer PDFs, such as those from Microsoft Print to PDF, embed fonts as Identity-H CID fonts and strip their names (`CIDFont+F1`). Their ToUnicode map gives the Latin-1 character for each glyph, which again is the legacy code. Text extractors therefore show `¨sÁUÀ` for ಭಾಗ.
3. **Glyphs named by their code.** Acrobat Distiller converts TrueType fonts to Type 1C with codes renumbered from 1 and a `Differences` list that names each glyph `G<n>`, where n is the font's own code. The Karnataka Acts of 2001 to 2003 embed AkrutiKndPadma this way.
4. **Renumbered subsets.** Some PDF makers renumber a subset's codes from `!` upward. When the font keeps a ToUnicode map that gives the Latin-1 characters back, the legacy codes can be read from the map. Otherwise the codes say nothing about the glyphs.

## What the tool does

- It reads each page's content with [lopdf](https://crates.io/crates/lopdf) 0.47 and walks the text operators, tracking the current font, the text matrix, the transformation matrix (`cm`, and the `Matrix` of form XObjects) and the font's glyph widths (`Widths`, or `W` for CID fonts).
- A font is legacy if its name, without the subset tag and style suffix, is a known Nudi, BRH Kannada or AkrutiKndPadma font, or one given with `--legacy-font`; the name gives its layout. In a font named as legacy, glyph names `G<n>` in a `Differences` list are read as code n. Its strings are read as raw codes for 1-byte fonts, and through the font's encoding or ToUnicode map for CID fonts. When the raw codes of a 1-byte font do not convert but its ToUnicode map gives text that does, the map is used. For text of fewer than 10 syllables, the map is used when only its reading looks like legacy codes and converts with no problems.
- A font with a hidden name is legacy if at least 3 in 10 of its visible characters are Windows-1252 characters above ASCII and its text converts, in Nudi or else in Akruti, with problems in at most 1 syllable in 10. Text of fewer than 10 syllables is too short to judge by its problem rate: it is converted when it converts with no problems, and kept as it is otherwise.
- A 1-byte font with a `Differences` list and no base encoding other than WinAnsi is read through that list on a Windows-1252 base. WinAnsi glyph names and `uniXXXX` names give their characters, names lopdf knows, such as `/fi`, are read by lopdf, and other names, such as `/c129`, keep their code. lopdf rejects a list with a name it does not know, and reads a list with no base encoding on StandardEncoding, which misreads legacy codes; so the tool reads the list itself. A list on another base, such as MacRoman, is left to lopdf.
- Strings drawn in pieces are joined. Within a text object, a space goes between 2 strings only when the gap between them is wider than a fifth of the font size, or a `TJ` adjustment moves more than a fifth of an em. A gap before a glyph that cannot start a syllable, such as a head stroke or a subscript, is spacing between glyphs, as justified PageMaker text has, and adds no space. Text in different layouts on 1 line is converted piece by piece. A new line starts when a string's baseline on the page is more than half the font size from the last one's. Each text object starts from the identity text matrix, as the PDF specification says. `q` and `Q` save and restore the font and the transformation matrix. Text in form XObjects is read with the form's matrix; a form is drawn inside its own `q` and `Q`, so the page's font and saved states come back after it.
- Fonts with the same name and the same encoding and ToUnicode map are judged together; fonts that share only a name are judged apart.
- The `k` Nudi fonts, such as Nudi 04 k, give Kannada digits.
- Text in other fonts is kept as the PDF's own encoding gives it.
- Pages are separated by a form feed.
- A page that cannot be read, and strings drawn with no font or a font the page does not define, are reported as `not converted` with the page number. The other pages are still converted. A page may run at most 1 million operations, counting each time a form is drawn, and a ToUnicode map may take at most 131,072 entries, so a hostile file cannot hang the tool or fill memory.

These fonts are reported as `not converted`, with their text left out:

- a font named as Nudi or BRH whose text does not convert, which happens when codes were renumbered;
- a font that looks like legacy Kannada but converts in no known layout, such as Shree-Lipi or AkliteKndPadmini;
- a font whose text is mostly rare symbols such as `! # $ % & *`, which means renumbered codes with no usable map.

## Measured on public PDFs

Run on public PDFs from Karnataka government sources, kept outside the repository:

| Document | Fonts | Result |
|---|---|---|
| 2002 budget summary, [BGLAN2002.pdf](http://web.archive.org/web/20090410021533/http://www.kar.nic.in/finance/bud2002-kan/BGLAN2002.pdf), 1 page | BRH Kannada subsets, WinAnsi | 37 Kannada lines, no problems; numbers kept in place |
| 2002 budget by department, [vibhagavaru.pdf](http://web.archive.org/web/20090410031702/http://www.kar.nic.in/finance/bud2002-kan/vibhagavaru.pdf), 1 page | NudiAkshara, BRH Kannada, BRH Kasturi | 24 Kannada lines, no problems |
| Advocate General recruitment notice, 2015, [PDF](http://web.archive.org/web/20150922093911/http://www.karnataka.gov.in:80/2015/RECRUITMENT_OR_STENO_and_TYPIST.pdf), 14 pages | Nudi 04 k, Identity-H and WinAnsi | 365 Kannada lines, 2 malformed syllables, both where a space is drawn inside a word |
| High Court notification in the Karnataka Gazette, 10 February 2025, [PDF](http://web.archive.org/web/20250421085341/https://karnatakajudiciary.kar.nic.in/newwebsite/common_folder/notification/8187.pdf), 21 pages | CID fonts with stripped names, placed with `cm` | 399 Kannada lines found and converted; 2 unknown glyphs and 5 malformed syllables, most where the source has a doubled vowel sign such as ದೆೆ |
| 2002 budget highlights and speech, [bh2002k.pdf](http://web.archive.org/web/20040822015443/http://www.kar.nic.in:80/finance/bud2002-kan/bh2002k.pdf) and [bs2002-k.pdf](http://web.archive.org/web/20060517033426/http://kar.nic.in:80/finance/bud2002-kan/bs2002-k.pdf), 85 pages | AkrutiKndPadma TrueType subsets, WinAnsi | 2,153 Kannada lines; 1 unknown glyph and 4 malformed syllables, all typing errors in the source |
| Karnataka Rent Act, 2001, [PDF](https://web.archive.org/web/20090409214440/http://dpal.kar.nic.in/ao2001/(34%20of%202001)%20Rent%20Act-(K).pdf), 76 pages, and 5 other Acts of 2001 to 2003 | AkrutiKndPadma Type 1C, glyphs named `G<n>`; BRH Kannada | Converted; the issues left are the source's own, and 19 lines in AkliteKndPadmini in 1 Act are reported as another layout |
| Unicode L2/03-068 from Karnataka DIT, [PDF](https://www.unicode.org/L2/L2003/03068-kannada.pdf), 16 pages | 5 Nudi01k subsets, renumbered by pdfFactory | 47 Kannada lines read through the ToUnicode map of 1 subset, 1 malformed syllable; the other 4 subsets reported as renumbered |
| Legislative Assembly member page, [13mem/50.pdf](http://web.archive.org/web/20150427105120/http://kla.kar.nic.in/13mem/50.pdf), 2 pages | Renamed TrueType subset with a `Differences` list, made by Ghostscript | Reported as another layout; not converted |

The Gazette notification also has a heading in a Unicode font whose ToUnicode map is broken; it comes out garbled, as it does in other extractors. That is not a legacy font.

## What does not work

- **Renumbered subsets.** The codes can only be recovered from the embedded font itself: its `cmap` and `post` tables, or the glyph outlines matched against a known Nudi font. That means reading the font program, so it is left for a decision. For Nudi the glyph shapes are known, so a later tool could match outlines; the font files must be used under their licenses.
- **Other layouts.** Shree-Lipi and AkliteKndPadmini have no table yet; see [shree-lipi.md](shree-lipi.md). Akruti is described in [akruti.md](akruti.md).
- **Reading order.** Lines follow the order the PDF draws them. Multi-column pages and tables come out in drawing order, which is usually right for government documents and sometimes not.
- **Scanned pages.** Pages that are images have no text. They need OCR, which this crate does not do.
- **Short legacy text in a mostly Latin font.** A font is judged by all its text. The Gazette's page header ¨sÁUÀ (ಭಾಗ) is drawn in a font that mostly shows Latin text, so it stays as it is.
- **Words in separate text objects.** Gaps between strings are measured only within a text object, since glyph widths in real PDFs are not exact enough to measure across them. 2 words drawn in separate text objects with no space character between them run together.
- **Rotated text.** Line breaks are measured on the page, but gaps are measured along the text's own x axis, and lines are taken as horizontal. Rotated text may get extra line breaks.
