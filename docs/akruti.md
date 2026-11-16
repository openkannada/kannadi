# Akruti: how the table was made, and how well it converts

Status: `Layout::Akruti` converts text in AkrutiKndPadma and its Bold, Demi and Demib weights, in both directions. The table is [akruti.tsv](../crates/kannadi/data/akruti.tsv). A Kannada reader's check of the fixture lines is pending.

## The layout

Akruti stores Kannada the way Nudi does: a consonant shape, then the head stroke or a vowel sign, then subscripts, then the arkavattu. The codes are different. The head stroke is E2, ಾ is D6, ೆ is EA, the length mark ೕ is A9, the virama is D2 and the arkavattu is CB. ಮ is ವ plus a tail (C0 E2 B0), and ಯ is a base of its own with the same tail. A few glyphs draw 2 subscripts at once, such as ್ಟ್ರ (E5), or a subscript with its vowel sign, such as ್ಕೃ (AF).

Typists often put a space before ೃ, ೖ and the subscript ಯ, which draw under the glyph before them: ಸಂಸ್ಕ ೃತಿ. The table reads a space followed by one of these as part of the syllable before it. They also typed ೆ followed by ೃ for ೈ (ವೆೃರ್ for ವೈರ್), which looks the same on the page; the table reads it as ೈ.

## Sources

No code page is published for Akruti. The table was derived from public documents, all fetched from the Internet Archive:

- Karnataka Acts and Ordinances in Kannada from `dpal.kar.nic.in`, 2001 to 2003, made with PageMaker 6.52 and Acrobat Distiller 5.0. Some embed the whole AkrutiKndPadmaDemi and AkrutiKndPadmaBold font programs. Their text uses codes 1 to N with a Differences list whose glyph names `G<n>` give each glyph's code n.
- The 2002 budget highlights, [bh2002k.pdf](http://web.archive.org/web/20040822015443/http://www.kar.nic.in:80/finance/bud2002-kan/bh2002k.pdf), which embeds TrueType subsets of AkrutiKndPadma Bold and Demi whose codes are the bytes the page shows.

## Method

1. Render every glyph of the embedded fonts, with its advance width, and read it.
2. Match each glyph against glyphs shaped with HarfBuzz from Kannada Sangam MN and Noto Sans Kannada, as a second opinion, for subscripts above all.
3. Count, over all the text, which glyphs follow each glyph. A shape followed by the head stroke is a base; one never followed by it is a letter.
4. Convert all the text, list every unknown glyph and malformed syllable, and read each against the rendered page. Every difference became a table entry or was confirmed as the source's own typing error.
5. Compare Bold and Demi code by code. They draw the same letter at every code.

Glyphs the fonts have but no document used are left out of the table. They are listed in its header and are reported as unknown if they appear.

## Results

| Measure | Result |
|---|---|
| Documents with Akruti text | 34: 33 Acts and Ordinances of 2001 to 2003, and the 2002 budget highlights |
| Text lines and glyphs | 14,999 lines, 807,491 glyphs |
| Unknown glyphs | 5 |
| Malformed syllables | 17 |
| Fixture lines read against the page image | 125 lines from 5 documents, all equal |
| Words that encode back to the bytes typed | 82,885 of 83,816 (98.9%) |

By font, over the same text:

| Font | Documents | Glyphs | Unknown | Malformed |
|---|---|---|---|---|
| AkrutiKndPadmaDemi | 34 | 766,835 | 5 | 17 |
| AkrutiKndPadmaBold | 31 | 23,849 | 0 | 0 |
| AkrutiKndPadmaDemib | 11 | 12,546 | 0 | 0 |
| AkrutiKndPadmaDemii | 23 | 4,176 | 0 | 0 |
| AkrutiKndPadmaBoldi | 6 | 84 | 0 | 0 |

Unknown and malformed counts are per line of text as the PDF draws it. A space the page leaves by moving the text, with no space character, is counted as a space.

The remaining unknown glyphs and malformed syllables were read against the page. They are the source's own errors, such as a stray subscript, a doubled ಮ tail, a mahaprana mark on ಾ, ೃ typed for ೖ, ು typed before ೂ, and ೃ carried to the next line, or a syllable split by the PDF across 2 lines.

The Repealing and Amending Act of 2003 also uses AkliteKndPadmini, which is a different layout. Its 19 lines are not converted. The Acts on the same site from 2004 on use Nudi.

The fixture is [akruti.tsv](../crates/kannadi/tests/fixtures/akruti.tsv): 125 lines from the Rent Act, State Universities Act, Reservation Act and Hindu Religious Institutions Act of 2001 and the 2002 budget highlights, with the source URL and page of each.

## Limits

- Unicode cannot tell an explicit virama from a conjunct without ZWNJ. ಆರ್ಡಿನೆನ್ಸ್ ಗಳ typed with a visible virama reads back as ಆರ್ಡಿನೆನ್ಸ್ಗಳ.
- The encoder writes the arkavattu after the length mark ೕ; typists of these Acts stored it before. Both read back the same.
- The encoder writes each subscript on its own; it does not use the glyphs that draw 2 at once.
- Akruti has no glyphs for ಙ, ಝ, ಱ and ೞ, and none for ಞ with ಿ.
- Only AkrutiKndPadma was seen in documents. Other Akruti Kannada fonts may use other layouts.
