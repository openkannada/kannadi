# Kan-Eng: how the table was made, and how well it converts

Status: `Layout::KanEng` converts text in the Kan-Eng fonts, such as Kan-EngPraja and Kan-EngDeepa, in both directions in the library; the command-line tool and the editor page convert it to Unicode. The table is [kan-eng.tsv](../crates/kannadi/data/kan-eng.tsv). A Kannada reader's check of the fixture lines is pending.

## The fonts

The Kan-Eng fonts are bilingual: Latin letters and digits sit at their ASCII codes, and Kannada sits at the ASCII symbol codes `$`, `%`, `&`, `<`, `]`, `^`, the backtick, `{`, `}` and `~`, and from 81 up. Karnataka's budget papers of 2002 to 2014 on `kar.nic.in/finance` set their Kannada tables in them, mostly in Kan-EngPraja and Kan-EngDeepa, with a few in Kan-EngKarn, Kan-EngVani and Kan-EngUsha. `legacy::font_layout` gives every font whose name starts with Kan-Eng this layout. The table was built from Kan-EngPraja and Kan-EngDeepa; text in the others converts as cleanly: the 2003 errata, set in Kan-EngKarn, Kan-EngVani and Kan-EngPraja, gives 0 unknown glyphs and 3 malformed syllables.

## The layout

A consonant is a shape, then the head stroke (DC) or a vowel sign, then subscripts. ಾ is DE, ು is E1, ೂ is E3, ೆ is E5, the length mark ೕ is E0, ೖ is E6, ೃ is ED, ೌ is E7, the virama is DD, the arkavattu is C0, ಂ is F9 or 8C, and ಃ is 8D. ಪ, ಫ and ವ take their own ು (E2) and ೂ (E4). FB is the mahaprana mark that turns ಬ into ಭ and ದ into ಧ. Many consonants with ಿ have a glyph of their own, such as ಕಿ at 94 and F8. AD is the subscript ಣ.

ಮ is ವ with the head stroke and the ು tail (C9 DC E1). ಯ starts with the circle that also draws ಂ, then the tail, the head stroke and the tail again (F9 E1 DC E1). ಮೊ was typed 2 ways, C9 E5 E1 DE and C9 E5 E3; the encoder writes the first.

Several glyphs sit at 2 codes, such as ಂ at F9 and 8C, the subscript ಕ at 7E and 92, and ಕಿ at 94 and F8. The documents use both; the encoder writes 1 of them.

## Sources

The PDFs were fetched from the Internet Archive: 45 budget papers of 2002 to 2014 that embed Kan-Eng fonts, such as the [2003 errata](https://web.archive.org/web/20030812181515/http://www.kar.nic.in:80/finance/bud2003/errata2003-04.pdf), the [2005 budget overview](https://web.archive.org/web/20051212212136/http://kar.nic.in:80/finance/bud2005/budover05-06.pdf) and the [2002 guarantees statement](https://web.archive.org/web/20060517074014/http://kar.nic.in:80/finance/bud2002/guar.pdf). They were made with Acrobat PDFMaker for Excel and Acrobat Distiller, which embed each font as a TrueType subset with WinAnsi encoding.

Acrobat cannot put a glyph at a code Windows-1252 leaves undefined (81, 8D, 8F, 90 and 9D), or at AD, in a WinAnsi font, so it moves those glyphs into a second, CID-keyed copy of the font. The copy's ToUnicode map gives the undefined codes back as U+0081 and so on, and AD as U+2212. In the Kan-Eng papers the copies held only those codes; other fonts' copies also hold B5, as U+03BC (see [pdf.md](pdf.md)). The copy has no cmap, but its glyph IDs are the code minus 30 (decimal: code 0x81, 129, is glyph 99), as in every subset's own cmap from 7E to C0, which confirms each code. Each subset's cmap table is cut short, so the other glyphs were read from its Macintosh table.

## Method

1. Extract each paper's Kannada lines with their codes in drawing order, joining the glyphs of the CID copy back into the words they belong to.
2. Render each glyph of the embedded subsets at its code, and read it.
3. Settle each code from the words it appears in. 81 (ಆ), 9D (ಚ), 90 (the subscript ಮ) and AD (the subscript ಣ) were read from ಆಯವ್ಯಯ, ವೆಚ್ಚ, ಚೆನ್ನಮ್ಮ, and ಸಣ್ಣ, ಉಣ್ಣೆ and ಕೃಷ್ಣ. 8D (ಃ) was read by its shape; its 1 use is a typing error.
4. Convert all the lines, and read every unknown glyph and malformed syllable against the codes rendered with the embedded glyphs.
5. Pick fixture lines that cover the table, render each next to its conversion, and read them side by side.

## Results

| Measure | Result |
|---|---|
| Distinct Kannada lines from the 45 papers, in drawing order | 833 lines, 21,455 glyphs: 1 unknown glyph, 11 malformed syllables |
| Fixture lines read against the embedded glyphs | 118 lines from 36 documents, all equal |
| Fixture words that encode back to the bytes typed | 340 of 373 (91.2%) |
| All words that encode back to the bytes typed | 1,895 of 2,295 (82.6%); the rest use the other code of a glyph at 2 codes, or put a subscript before the head stroke |
| Whole fixture lines that `best_layout` gives another layout | 0 of 118 |
| The 45 papers through `kannadi to-unicode`, all their fonts | 416,411 Kannada characters, 810 unknown glyphs, 3,185 malformed syllables; before this layout, 382,212, 2,199 and 8,491 |

The problems left in the 833 lines were read against the glyphs: words the source typed wrongly, a syllable a table cell split, and the 1 use of ಃ. The `kannadi to-unicode` figures cover every font in the papers, Nudi B-Akshara among them, whose PDFMaker tables draw some syllables off the line.

`kannadi to-unicode` reads these papers with 2 rules added to the PDF reader with this layout: the CID copy's glyphs join the words around them, and `T*` stays on the baseline when Distiller sets a leading of 0. Without them, words such as ವೆಚ್ಚ lost their ಚ, and lines broke inside syllables.

The fixture is [kan-eng.tsv](../crates/kannadi/tests/fixtures/kan-eng.tsv), with the source URL of each line.

## Limits

- Letters the documents do not use, such as ಊ, ಔ, ಙ, ಛ, ಝ and ಞ as full letters, have no entry. The encoder reports them as unencodable.
- Some forms the encoder writes appear in no document, so they are untested against a page: ಭ with no vowel sign (BE FB DC) and ಮೋ (C9 E5 E1 DE E0; the 1 document use is C9 E5 E3 E0).
- A visible virama typed before the next consonant, as in ಕೌನ್ಸಿಲ್‌ಗಳು, comes out with no ZWNJ, so Unicode fonts draw a conjunct there. The other layouts do the same.
- Text drawn in a rotated table cell keeps its order only when the PDF draws it in reading order.
