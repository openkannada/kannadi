# Nudi B-Akshara: how the table was made, and how well it converts

Status: `Layout::NudiB` converts text in Nudi B-Akshara (NudiB-Akshar), the bilingual Nudi font, in both directions in the library; the command-line tool and the editor page convert it to Unicode. The table is [nudi-b.tsv](../crates/kannadi/data/nudi-b.tsv). A Kannada reader's check of the fixture lines is pending.

## The font

Nudi B-Akshara draws Latin letters, digits and punctuation at their ASCII codes and Kannada from 81 up, so 1 font sets both English and Kannada. Its Kannada codes differ from the other Nudi fonts', which put Kannada at the ASCII codes, so it needs a table of its own. Earlier versions of this crate read NudiB-Akshar in the Nudi layout; `legacy::font_layout` now gives it `Layout::NudiB`.

The layout was first found as an unnamed font on the 13th Karnataka Legislative Assembly's member pages. Ghostscript had replaced its name with `TTE10AECA0t00` when the pages were made. The same glyphs appear, under the name NudiB-Akshar, in the Karnataka budget papers: the 2011 budget speech's subset has the same outline at 78 of the 82 codes both use.

## The layout

It stores Kannada the way the other layouts do, at its own codes: a consonant shape, then the head stroke (D0) or a vowel sign, then subscripts, then the arkavattu (FE). ಾ is D1, ು is D4, ೂ is D6, ೆ is D9, the length mark ೕ is D3, ೖ is DA, ೃ is D8, ೌ is DB, the virama is FD, ಂ is 81 and ಃ is 82. ಪ, ಫ and ವ take their own ು (D5) and ೂ (D7). B0 is the mahaprana mark that turns ಬ into ಭ and ದ into ಧ. ಣಿ is ಣ with a separate ಿ (D2).

ಮ is ವ with the head stroke and a tail (C0 D0 D4); ಮಾ and ಮ್ use another tail (A6) before ಾ or the virama. ಯ starts with the small circle that also draws ಂ, then that tail, the head stroke and the ು tail (81 A6 D0 D4). ಝ is ರ with a hook (A5) and the ು tail. ಋ is 89 with the ು tail.

## Sources

All fetched from the Internet Archive:

- The 5 member pages of the 13th Assembly that the Archive holds: [13mem/50.pdf](http://web.archive.org/web/20150427105120/http://kla.kar.nic.in/13mem/50.pdf), [13mem/75.pdf](http://web.archive.org/web/20160304030357/http://kla.kar.nic.in/13mem/75.pdf), [assembly/member/13mem/40.pdf](http://web.archive.org/web/20140929094100/http://kla.kar.nic.in/assembly/member/13mem/40.pdf), [52.pdf](http://web.archive.org/web/20150317110005/http://kla.kar.nic.in/assembly/member/13mem/52.pdf) and [62.pdf](http://web.archive.org/web/20160929091548/http://kla.kar.nic.in/assembly/member/13mem/62.pdf), made with PScript5 and Ghostscript 8.15 in October 2011. Ghostscript renumbered the font's codes from 1 but kept a `Differences` list whose glyph names (`currency`, `Eth`, and `c129` for a code with no standard name) give each glyph's original code.
- 18 Karnataka budget papers of 2007 to 2014 on `kar.nic.in/finance` and 4 Assembly papers that name the font NudiB-Akshar, among them the [2011 budget speech](http://web.archive.org/web/20110304213411/http://www.kar.nic.in:80/finance/bud2011/bspkan2011.pdf), the demand books and the statements of expenditure.

The 233 member pages of the 14th Assembly use other Nudi fonts.

## Method

1. Turn each PDF's codes back into the original ones through its `Differences` list.
2. Render each glyph of the embedded subsets at its original code, with the PDF's widths, and read it.
3. Settle each code from the words it appears in: ರಹೀಮ್ ಖಾನ್ and ಹೆಸರು on the first member page fix the head stroke, ಾ, ೆ, ೕ and the virama; ವೆಚ್ಚ, ವಿಜ್ಞಾನ and ಬಗ್ಗೆ in the budget papers fix the subscripts ಚ, ಞ and ಗ. 8F (ಓ) and 9D (ಛ) are in no embedded subset and were read from their words (ಓದುವ, ಓವರ್; ಕಛೇರಿ, ಛಾಯಾಚಿತ್ರ).
4. Convert all the text, and read the unknown glyphs and malformed syllables against the codes rendered with the embedded glyphs.

## Results

| Measure | Result |
|---|---|
| Member pages | 161 distinct Kannada lines, 4,569 glyphs: 0 unknown glyphs, 0 malformed syllables |
| Budget and Assembly papers | 10,214 distinct lines, 639,208 glyphs: 65 unknown glyphs, 113 malformed syllables |
| Fixture lines read against the embedded glyphs | 191 lines from 15 documents, all equal |
| Fixture words that encode back to the bytes typed | 552 of 577 (95.7%) |
| Whole fixture lines that `best_layout` gives another layout | 0 of 191 |

The remaining problems read against the glyphs were the documents' own: a mahaprana mark typed on ವ, a word split at a line end, a vowel sign at the start of a line. The budget papers' lines exclude subsets that the PDF maker renumbered with no `Differences` list, whose codes are not the font's.

Many budget papers, made with Acrobat Distiller or an Oracle PDF driver, have no glyph for code 81, which Windows-1252 leaves undefined. On those pages ಂ shows as a gap (ಸ ಖ್ಯೆ for ಸಂಖ್ಯೆ) and ಯ without its circle; the text keeps the gap, as the page does, and the table reads ಯ without its circle.

Single words often read cleanly in Nudi too, as the 2 layouts share many shapes at nearby codes, so `best_layout` needs a line or more of text to tell them apart.

The fixture is [nudi-b.tsv](../crates/kannadi/tests/fixtures/nudi-b.tsv), with the source URL of each line.

## Limits

- Letters the documents do not use, such as ಙ and the subscript ಶ, have no entry; the encoder writes a consonant whose subscript has no glyph as a half form with a visible virama.
- Budget tables made with PDFMaker for Excel draw some syllables a little off the line, and the PDF reader splits those words across lines, so they read as malformed.
