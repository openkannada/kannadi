# PDF fixtures

`unicode-sans.pdf` and `unicode-serif.pdf` hold the same 4 lines of Unicode Kannada and English, printed to PDF by Chrome 157 from an HTML page set in the Noto Sans Kannada and Noto Serif Kannada fonts in `web/src/fonts`. Chrome embeds those variable fonts as Type 3 fonts with a ToUnicode map, and writes each conjunct as an `/ActualText` span. The text was written for kannadi.
