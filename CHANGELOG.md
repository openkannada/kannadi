# Changelog

## Unreleased

### Added

Library (`kannadi`):

- Akshara segmentation (`aksharas`, `split`, `count_aksharas`), including conjuncts, nukta, ZWJ and ZWNJ after a virama, and Old Kannada letters. `Akshara::is_well_formed` finds stray signs, signs on vowel letters and doubled vowel signs.
- Normalization (`normalize`): NFC, look-alike vowel letters, repeated marks. `fix_zero_as_anusvara` for converted and scanned text.
- Modernization (`modernize`): ೞ to ಳ, ಱ to ರ, ೝ to ನ್.
- Transliteration between Kannada and ISO 15919, IAST and Baraha phonetic, lossless in both directions for Kannada text.
- Kannada numerals: digit conversion, formatting and parsing.
- Nudi and Baraha ANSI to Unicode (`legacy::nudi_to_unicode`), with a glyph table derived from the font files and reports of unknown glyphs and malformed syllables.
- Unicode to Nudi and Baraha ANSI (`legacy::unicode_to_nudi`), writing each akshara in the order typists stored it, with reports of unencodable characters.
- Akruti to Unicode and back (`legacy::to_unicode` and `legacy::from_unicode` with `Layout::Akruti`), with a glyph table derived from the fonts embedded in Karnataka Acts and checked on 908,075 glyphs from 35 documents. `legacy::Layout`, `UnknownLayout` and `syllable_starts`.
- Shree-Lipi to Unicode and back (`Layout::ShreeLipi`), with a glyph table derived from the Shree-Kan-0850 font the Udayavani newspaper served with its web pages, checked on 233,346 glyphs from 173 pages. `legacy::font_layout` gives Shree-Kan-0850 this layout; other Shree-Kan fonts stay legacy fonts with no table.
- Nudi B-Akshara to Unicode and back (`Layout::NudiB`), with a glyph table derived from the fonts embedded in Karnataka Legislative Assembly member pages, whose PDFs dropped the font's name, and in budget papers that name it NudiB-Akshar. `legacy::font_layout` gives NudiB-Akshar this layout; earlier it was read as Nudi. `Layout::draws_latin` for layouts whose fonts keep Latin letters at the ASCII codes.
- Kan-Eng to Unicode and back (`Layout::KanEng`), with a glyph table derived from the Kan-EngPraja and Kan-EngDeepa fonts embedded in Karnataka budget papers of 2002 to 2014, checked on 21,455 glyphs from 45 papers. `legacy::font_layout` gives every font named Kan-Eng this layout. The encoder puts the mahaprana mark on a letter when the layout draws the plain consonant only as a letter.
- `legacy::best_layout` breaks ties by fewer Latin letters, then fewer viramas.
- The encoder writes a consonant whose subscript the layout cannot draw as a half form with a visible virama.
- `chars::Class::is_mark`.
- Sound-alike keys (`phonetic::key`, `keys`, `latin_key`) at 3 levels, for fuzzy search and for suggesting Kannada words from loosely typed Latin letters: "namaskara" and ನಮಸ್ಕಾರ share a medium key. `kannadi-wasm` exports `phoneticKey` and `latinKey`, so the editor page can use them.

- Likely typos (`typos::typos`, `typos::fix_typos`): ರೃ at the start of a syllable is flagged with ಋ as the fix, as in ನೈರೃತ್ಯ for ನೈಋತ್ಯ. `kannadi-wasm` exports `typos`, with UTF-16 offsets, for editors that mark them.

Command-line tool (`kannadi-cli`):

- `kannadi to-unicode` and `kannadi to-nudi` for files and folders, with a per-file report of problems and `--strict`.
- Plain text, including UTF-16 and Windows-1252 input.
- HTML: converts only text set in a legacy font, found from `style`, `<style>` rules and `<font face>`.
- DOCX: converts only runs in a legacy font, following run, character and paragraph styles.
- PDF: extracts text and converts text in legacy fonts, including fonts renumbered behind a ToUnicode map.
- PDF: Ghostscript's glyph names `c<n>` are read as code n; strings drawn several times a fraction of a point apart, to look bold, are read once; short text in a font with a hidden name is not taken as a layout that draws Latin.
- PDF: `T*`, `'` and `"` move down by the text leading, as `TL` and `TD` set it, so the parts of a line that Acrobat Distiller draws with a leading of 0 stay on 1 line.
- PDF: the CID copy Acrobat makes of a WinAnsi font, for glyphs at codes Windows-1252 leaves undefined and at AD and B5, is read back to those codes, so its glyphs join the words around them. Named legacy CID fonts have their ToUnicode map read.
- Akruti in every format: font names give each run's layout in HTML, DOCX and PDF; `to-unicode --layout akruti` for text files; `--legacy-font "NAME=akruti"`. In PDFs, glyph names `G<n>` in renumbered lists are read as code n, fonts with hidden names are judged in each layout, and a gap that would break a syllable adds no space.

Editor page and bindings:

- `kannadi-wasm`: WebAssembly bindings for conversion in both directions, transliteration, normalization, segmentation, syllable checks and fixes, with offsets in UTF-16 code units.
- The editor page reads Akruti: DOCX and HTML text by its font, pasted and plain text in the layout that converts best (`legacy::best_layout`, which breaks ties by fewer viramas), chosen once per paste, file or selection. `kannadi-wasm`: `toUnicode` with a layout name, and `bestLayout`.
- A static Kannada editor in `web/`: open TXT, DOCX and HTML, converting only text set in Nudi or Baraha fonts; inline marks for unknown glyphs and malformed syllables with checked fixes; Baraha and ISO 15919 typing; normalize and modernize; akshara and word counts; save as DOCX, TXT, HTML or Nudi text; autosave in the browser. Self-hosted Noto Sans Kannada and Noto Serif Kannada.
- Landing pages for Nudi to Unicode, Unicode to Nudi, Baraha to Unicode, Kannada typing and English to Kannada typing, built from 1 template with JSON-LD, Open Graph cards, `llms.txt`, and a sitemap when the site URL is set.

Data and tests:

- A benchmark corpus of 207 lines from 31 Karnataka government pages of 2002, with a data card.
- Golden fixtures from Karnataka government pages and Acts, property tests, and a differential test against vidyut-lipi.

Font tools:

- `tools/fontqa`: HarfBuzz shaping tests for Kannada fonts, results for 41 fonts (146 faces) with a data card, proof sheets, and fixes for broken `knd2` tables, broken `morx` tables and missing ೄ, ೢ and ೣ. `measure` and `gate` commands, and a mean stroke width gate.
- `tools/glyphdraft`: drafts U+0CDC, U+0CDD and U+0CF3 into a FontForge source, matched to the font's stems, weight, heights and spacing.
- fontqa `export`: the test strings as FontBakery shaping tests (forbidden glyphs, rare characters apart, collisions) and Shaperglot `kn_Knda` checks (conjuncts, arkavattu), with draft upstream texts. fontqa `dashboard`: a static coverage page built from the results.

Project:

- MIT license, `CITATION.cff`, `AGENTS.md` and `CONTRIBUTING.md`.

### Fixed

- fontqa outline flattening: a TrueType quadratic with several off-curve points is split at its implied on-curve points, and a closed contour with no on-curve point is read. Stroke measurements on TrueType fonts were slightly off before.
- fontqa manifest: Navilu is OFL-1.1 with Reserved Font Name Navilu, not GPL.
- glyphdraft: side bearings come from the median of the font's consonants, not from KA, whose bearings can be an outlier.
