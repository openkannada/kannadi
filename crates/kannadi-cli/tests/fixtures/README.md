# Fixtures

## PDF

`unicode-sans.pdf` and `unicode-serif.pdf` hold the same 4 lines of Unicode Kannada and English, printed to PDF by Chrome 157 from an HTML page set in the Noto Sans Kannada and Noto Serif Kannada fonts in `web/src/fonts`. Chrome embeds those variable fonts as Type 3 fonts with a ToUnicode map, and writes each conjunct as an `/ActualText` span. The text was written for kannadi.

## DOC and RTF

`make-word.py` writes `legacy.docx` from lines of the library's glyph fixtures (`crates/kannadi/tests/fixtures/nudi.tsv` and `akruti.tsv`, short excerpts of Karnataka government documents with their sources there): runs set in Nudi 01 e, BRH Kannada, Nudi 04 k and AkrutiKndPadma, a paragraph style that sets Nudi 01 e, an English run, a Unicode Kannada run, a table and a header. It writes `legacy.expected.txt`, the text kannadi should read, from the expected column of the same lines. LibreOffice 26.8.3.1 saved `legacy.doc` (MS Word 97 filter) and `legacy.rtf` from it, headless: `python3 make-word.py /Applications/LibreOffice.app/Contents/MacOS/soffice`.
