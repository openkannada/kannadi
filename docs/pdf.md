# Text from PDFs typeset in legacy fonts

Status: `kannadi to-unicode` reads PDF files and writes their text as `NAME.pdf.txt`, with text in Nudi and Baraha ANSI fonts converted to Unicode. This note says how it works, what was measured, and what does not work.

## How legacy text sits in a PDF

A PDF shows text with operators such as `Tj` and `TJ`, which give a string of codes in the current font. For a legacy font, those codes are the same glyph codes the font uses in a Word file or a web page, so they can go straight to `legacy::nudi_to_unicode`. The work is in finding the codes and the fonts:

1. **1-byte fonts with WinAnsi encoding.** Word and Distiller output from the 2000s embeds BRH and Nudi fonts this way, often as subsets named like `CCMNNO+BRHKannada`. The string bytes are the glyph codes.
2. **2-byte CID fonts with a ToUnicode map.** Newer PDFs, such as those from Microsoft Print to PDF, embed fonts as Identity-H CID fonts and strip their names (`CIDFont+F1`). Their ToUnicode map gives the Latin-1 character for each glyph, which again is the legacy code. Text extractors therefore show `¨sÁUÀ` for ಭಾಗ.
3. **Renumbered subsets.** Some PDF makers renumber a subset's codes from `!` upward and leave no useful ToUnicode map. The codes then say nothing about the glyphs.

## What the tool does

- It reads each page's content with [lopdf](https://crates.io/crates/lopdf) 0.47 and walks the text operators, tracking the current font, the text position and the font's glyph widths (`Widths`, or `W` for CID fonts).
- A font is legacy if its name, without the subset tag and style suffix, is a known Nudi or BRH Kannada font, or one given with `--legacy-font`. Its strings are read as raw codes for 1-byte fonts, and through the font's encoding or ToUnicode map for CID fonts.
- A font with a hidden name is legacy if at least 3 in 10 of its visible characters are Windows-1252 characters above ASCII and its text converts with problems in at most 1 syllable in 10.
- Strings drawn in pieces are joined. Within a text object, a space goes between 2 strings only when the gap between them is wider than a fifth of the font size, or a `TJ` adjustment moves more than a fifth of an em. A vertical move of more than half the font size starts a new line. `q` and `Q` save and restore the font, and text in form XObjects is read.
- Fonts with the same name and the same encoding and ToUnicode map are judged together; fonts that share only a name are judged apart.
- The `k` Nudi fonts, such as Nudi 04 k, give Kannada digits.
- Text in other fonts is kept as the PDF's own encoding gives it.
- Pages are separated by a form feed.

These fonts are reported as `not converted`, with their text left out:

- a font named as Nudi or BRH whose text does not convert, which happens when codes were renumbered;
- a font that looks like legacy Kannada but does not convert, which means another layout such as Akruti;
- a font whose text is mostly rare symbols such as `! # $ % & *`, which means renumbered codes with no usable map.

## Measured on public PDFs

Run on 8 public PDFs from Karnataka government sources, kept outside the repository:

| Document | Fonts | Result |
|---|---|---|
| 2002 budget summary, [BGLAN2002.pdf](http://web.archive.org/web/20090410021533/http://www.kar.nic.in/finance/bud2002-kan/BGLAN2002.pdf), 1 page | BRH Kannada subsets, WinAnsi | 37 Kannada lines, no problems; numbers kept in place |
| 2002 budget by department, [vibhagavaru.pdf](http://web.archive.org/web/20090410031702/http://www.kar.nic.in/finance/bud2002-kan/vibhagavaru.pdf), 1 page | NudiAkshara, BRH Kannada, BRH Kasturi | 24 Kannada lines, no problems |
| Advocate General recruitment notice, 2015, [PDF](http://web.archive.org/web/20150922093911/http://www.karnataka.gov.in:80/2015/RECRUITMENT_OR_STENO_and_TYPIST.pdf), 14 pages | Nudi 04 k, Identity-H and WinAnsi | 364 Kannada lines, 2 malformed syllables, both where a space is drawn inside a word |
| High Court notification in the Karnataka Gazette, 10 February 2025, [PDF](http://web.archive.org/web/20250421085341/https://karnatakajudiciary.kar.nic.in/newwebsite/common_folder/notification/8187.pdf), 21 pages | CID fonts with stripped names | 319 Kannada lines found and converted; 2 unknown glyphs and 14 malformed syllables, most where the source has a doubled vowel sign or a sign split across lines |
| 2002 budget speech PDFs, [bh2002k.pdf](http://web.archive.org/web/20040822015443/http://www.kar.nic.in:80/finance/bud2002-kan/bh2002k.pdf) and [bs2002-k.pdf](http://web.archive.org/web/20060517033426/http://kar.nic.in:80/finance/bud2002-kan/bs2002-k.pdf), 90 pages | AkrutiKndPadma | Reported as another layout; not converted |
| Unicode L2/03-068 from Karnataka DIT, [PDF](https://www.unicode.org/L2/L2003/03068-kannada.pdf), 16 pages | Nudi01k, renumbered by pdfFactory | Reported as renumbered; not converted |
| Legislative Assembly member page, [13mem/50.pdf](http://web.archive.org/web/20150427105120/http://kla.kar.nic.in/13mem/50.pdf), 2 pages | Renamed TrueType subset, renumbered by Ghostscript | Reported as unreadable; not converted |

The Gazette notification also has a heading in a Unicode font whose ToUnicode map is broken; it comes out garbled, as it does in other extractors. That is not a legacy font.

## What does not work

- **Renumbered subsets.** The codes can only be recovered from the embedded font itself: its `cmap` and `post` tables, or the glyph outlines matched against a known Nudi font. That means reading the font program, so it is left for a decision. For Nudi the glyph shapes are known, so a later tool could match outlines; the font files must be used under their licenses.
- **Other layouts.** Akruti and Shree-Lipi need their own tables; see [shree-lipi.md](shree-lipi.md).
- **Reading order.** Lines follow the order the PDF draws them. Multi-column pages and tables come out in drawing order, which is usually right for government documents and sometimes not.
- **Scanned pages.** Pages that are images have no text. They need OCR, which this crate does not do.
- **Words in separate text objects.** Gaps between strings are measured only within a text object, since glyph widths in real PDFs are not exact enough to measure across them. 2 words drawn in separate text objects with no space character between them run together.
- **Rotated or scaled text.** Position tracking assumes text runs left to right without rotation; rotated text may get extra line breaks.
