# Kannada text normalization profile

Version 1.0.0, draft of 19 November 2026.

Kannada text can be spelled in several ways that look the same on screen. A word typed with a joiner after its last letter, or with a vowel sign typed in 2 parts, does not match the same word typed plainly. Search misses it, a spell checker flags it, a dictionary cannot find it, and OCR scoring counts an error a reader cannot see.

This profile fixes 1 spelling for each case. Every rule is backed by counts from Kannada Wikipedia and Wikisource and, for joiners, by how HarfBuzz renders the text. [kannadi](https://github.com/openkannada/kannadi) is the reference implementation, and [normalization/vectors.tsv](normalization/vectors.tsv) holds the test vectors.

## Forms

The profile has 3 forms. Each one applies the rules of the forms before it.

| Form | What it does | Use it for |
|---|---|---|
| canonical | 1 spelling for text that renders the same, or that Unicode allows only 1 way | Storing and exchanging text. Safe to apply to anything. |
| legacy | Also repairs mistakes that legacy font converters and OCR make | Text converted from Nudi, Baraha and other legacy fonts, and OCR output |
| search | Also folds differences a reader ignores when searching | Search keys, dictionary lookup, deduplication. Never for stored text: it changes letters. |

To compare 2 texts that a reader would call equal, such as OCR output and its reference, normalize both to the canonical form. If the references do not record joiners consistently, also apply rule S2.

The rules apply to Kannada text: characters in U+0C80 to U+0CFF, and joiners and format characters next to them. Text in other scripts is changed by NFC only.

## Canonical form

| Rule | What changes | Example |
|---|---|---|
| C1 | Unicode Normalization Form C. 2-part vowel signs join, and a nukta goes before a virama. | ಕಿ + ೕ becomes ಕೀ |
| C2 | The vowel letters in Unicode's Table 12-32 that must not be built from parts: ಉ + ಾ is ಊ, ಒ + ೌ is ಔ, ಋ + ಾ is ೠ. | ಉಾರು becomes ಊರು |
| C3 | A nukta moves to directly after its consonant, over any vowel signs. | ಜೀ಼ becomes ಜ಼ೀ |
| C4 | A mark repeated directly after itself (vowel sign, virama, nukta, anusvara, visarga) is kept once. | ಅಥವಾಾ becomes ಅಥವಾ |
| C5 | A ZWNJ or ZWJ between a consonant and its virama is removed. After a ರ that starts its syllable, it becomes ZWJ. | ಮ‌್ಯೂಸಿಯಂ becomes ಮ್ಯೂಸಿಯಂ; ರ‌್ಯಾ becomes ರ‍್ಯಾ |
| C6 | ರ + virama + ZWJ before a consonant becomes ರ + ZWJ + virama, when the ರ starts its syllable. | ನೆದರ್‍ಲ್ಯಾಂಡ್ಸ್ becomes ನೆದರ‍್ಲ್ಯಾಂಡ್ಸ್ |
| C7 | Any run of joiners between a virama and a consonant becomes 1 ZWNJ, except the case of C6. | ರಾಜ್‍ಕುಮಾರ್ (ZWJ) becomes ರಾಜ್‌ಕುಮಾರ್ (ZWNJ) |
| C8 | Any other ZWNJ or ZWJ next to Kannada is removed: at the end or start of a word, after a vowel sign or vowel letter, between 2 consonants with no virama. A joiner between a consonant and the vowel sign or other mark after it stays. | ಆಫ್‌ becomes ಆಫ್ |

Rules C1 to C8 repeat until the text no longer changes. Removing a joiner can bring the 2 parts of a vowel sign together, which C1 then joins.

The canonical form keeps 2 joiner spellings, the ones Unicode defines for Kannada:

- **virama + ZWNJ + consonant** shows the virama instead of a conjunct: ಕನ್‌ನಡ. The Unicode Standard (section 12.8, "Explicit Virama") puts the ZWNJ "immediately after the encoded dead consonant".
- **ರ + ZWJ + virama + consonant** writes ರ in full with the next consonant below it, instead of the arkavattu: ರ‍್ಯ. This is the sequence in section 12.8, "Consonant Clusters Involving RA".

Old Kannada letters stay as they are: ೞ (U+0CDE, which Unicode names KANNADA LETTER FA by mistake and aliases LLLA), ಱ (U+0CB1), ೝ (U+0CDD, Unicode 14.0), the combining anusvara above right ೳ (U+0CF3, Unicode 15.0) and the archaic shrii U+0CDC (Unicode 17.0). They are distinct letters, and the search form folds them.

### Why the joiner rules hold

The joiner rules rewrite text into a spelling that renders the same. [normalization/render_check.py](normalization/render_check.py) shapes each rewrite with HarfBuzz 15.0.0 and compares the ink, the glyphs with outlines at their positions, in 6 fonts:

| Rule | Written | Becomes | Fonts that render both the same |
|---|---|---|---|
| C5 | ಮ ZWNJ ್ ಯ | ಮ ್ ಯ | 5 of 6 (not Tiro Kannada) |
| C5 | ಪ ZWJ ್ ಲ | ಪ ್ ಲ | 6 of 6 |
| C5 | ರ ZWNJ ್ ಯ | ರ ZWJ ್ ಯ | 5 of 6 (not Tiro Kannada) |
| C6 | ರ ್ ZWJ ಲ | ರ ZWJ ್ ಲ | 5 of 6 (not Kannada Sangam MN) |
| C7 | ಜ ್ ZWJ ಕ | ಜ ್ ZWNJ ಕ | 6 of 6 |
| C7 | ನ ್ ZWNJ ZWNJ ನ, and ZWJ with ZWNJ in either order | ನ ್ ZWNJ ನ | 6 of 6 |
| C8 | joiner after a word-final virama, after a vowel sign, between consonants, at a word start | nothing | 6 of 6 |
| kept | ಮ ZWNJ ೂ | ಮ ೂ | 4 of 6, so C8 leaves it |

The fonts are Noto Sans Kannada, Noto Serif Kannada, Tiro Kannada, Anek Kannada, Gari and Kannada Sangam MN. HarfBuzz treats ರ + virama + ZWJ like ರ + ZWJ + virama for Kannada, which C6 follows, picking the order the Unicode Standard gives. In the 2 exceptions, the font itself handles the sequence differently.

## Legacy form

| Rule | What changes | Example |
|---|---|---|
| L1 | A Kannada zero ೦ after a letter or sign, and not before a digit, becomes the anusvara ಂ. | ಬೆ೦ಗಳೂರು becomes ಬೆಂಗಳೂರು; ಕೊಠಡಿ೦೧ stays |
| L2 | A Latin o, O or 0 between a vowel sign and a Kannada letter becomes ಂ. | ಹೊoದಿದೆ becomes ಹೊಂದಿದೆ |
| L3 | A consonant cluster followed by ರ + virama + a vowel sign is read as an arkavattu typed after its consonant: ರ + virama moves to the start of the cluster. A virama directly before a vowel sign is never valid. | ಸಕರ್ಾರ becomes ಸರ್ಕಾರ; ಆಥರ್ಿಕ becomes ಆರ್ಥಿಕ |

These are guesses about intent, so the canonical form does not make them. Legacy fonts store the arkavattu after the consonant it sits on, and the anusvara looks like a zero, so naive converters and OCR produce exactly these spellings.

## Search form

| Rule | What changes | Example |
|---|---|---|
| S1 | Old letters fold to modern ones: ೞ to ಳ, ಱ to ರ, ೝ to ನ್, ೳ to ಂ, U+0CDC to ಶ್ರೀ. | ಬಾೞ್ becomes ಬಾಳ್ |
| S2 | ZWNJ, ZWJ, zero width space, soft hyphen, word joiner and byte order mark next to Kannada are removed. | ಕನ್‌ನಡ becomes ಕನ್ನಡ |
| S3 | The nukta is removed. | ಜ಼ೀರೋ becomes ಜೀರೋ |

After S1 to S3 the legacy and canonical rules run again, since S1 can turn ೝ into a letter they apply to.

## Not normalized

These were counted and left alone, because a rule would change text that is correct:

- **A colon after a Kannada letter**, 271,741 times in Wikipedia, is not the visarga ಃ (24,491 times). Almost all are real colons, as in ಉದಾ: for "for example".
- **Dandas.** The Devanagari danda । (1,393) and the vertical bar after a word (7,463, most of them in wiki tables) are left as written.
- **2 different vowel signs in a row**, 624 times, have no single intended spelling. The commonest are stray ೂೀ at a word start.
- **A vowel sign or virama with no letter before it**, 2,133 times. Tools should report it as malformed, as kannadi's `aksharas` does.
- **Telugu letters next to Kannada**, 48 times, mostly OCR mistakes between look-alike letters. Report them.
- **Latin digits** and Kannada digits are both correct.

## Evidence

Counts are over the Kannada Wikipedia dump of 1 November 2026 (`knwiki` pages-articles, 781 MB of XML, 166,947,879 Kannada characters, newest revision 2026-11-01T14:00:28Z) and the Kannada Wikisource dump of 1 December 2023 (`wikimedia/wikisource`, 20231201.kn, 195 MB of text). They count changes made by `kannadi normalize --stats`; a run of several joiners counts once.

| Rule | Wikipedia | Wikisource |
|---|---:|---:|
| C1 nfc | 2 | 12 |
| C2 vowel-letter | 0 | 0 |
| C3 nukta-order | 501 | 429 |
| C4 repeated-mark | 1,711 | 906 |
| C5 joiner-before-virama | 744 | 4 |
| C6 non-reph-ra | 4,190 | 7,725 |
| C7 explicit-virama | 50,714 | 51,835 |
| C8 stray-joiner | 388,742 | 5,776 |
| L1 zero-as-anusvara | 12,453 | 4,912 |
| L2 latin-as-anusvara | 168 | 322 |
| L3 misplaced-reph | 117 | 9 |
| S1 old-letter | 935 | 8,330 |
| S2 invisible | 388,109 | 61,649 |
| S3 nukta | 8,657 | 855 |

Some context for these counts:

- **Joiners after a virama.** virama + ZWNJ occurs 705,040 times in Wikipedia and ZWNJ + virama 415 times. Of the virama + ZWNJ pairs, 327,530 are before a consonant, where they show the virama. Another 313,136 end a word, as in ಆಫ್‌ (6,978 times) and ಅಂಡ್‌ (3,004), where they change nothing. These, with ZWNJ runs such as ಕೇಂಬ್ರಿಡ್ಜ್ followed by 11 ZWNJs, make most of C8.
- **ZWJ for a visible virama.** virama + ZWJ occurs 45,759 times. 36,376 of these, not after ರ, are before a consonant, as in ರಾಜ್‍ಕುಮಾರ್ (665) and ಲರ್ನಿಂಗ್‍ನಲ್ಲಿ (1,060). Every font tested renders them as virama + ZWNJ, which is C7.
- **Non-arkavattu ರ.** ರ + virama + ZWJ occurs 4,948 times and Unicode's order, ರ + ZWJ + virama, 1,936 times. 314 of the 415 ZWNJ + virama pairs follow ರ, mostly in English loanwords, as in ರ‌್ಯಾಮ್‌ಸ್ಟೀನ್, where they render as ರ + ZWJ + virama.
- **NFC and Table 12-32.** MediaWiki stores text in NFC, so the dumps have almost no 2-part vowel signs. C1 and C2 matter for text from converters, OCR, PDFs and keyboards, which the test vectors cover.
- **Zero for anusvara.** ೦ follows a letter or sign 12,447 times in Wikipedia, 12,183 of them before another letter, as in ಎ೦ಬ (221) and ಒ೦ದು (148). 9 are numbers after a word, which L1 keeps.
- **Old letters.** Wikisource has ಱ 4,192 times and ೞ 4,138 times; Wikipedia has them 536 and 399 times. Neither dump has ೝ, ೳ or U+0CDC, which Unicode added in 2021 to 2025.

To reproduce:

```bash
cargo run --release -p kannadi-cli -- normalize --form search --stats knwiki-latest-pages-articles.xml > /dev/null
uv run --no-project --with uharfbuzz==0.57.0 python normalization/render_check.py FONT...
```

## Conformance

An implementation conforms to a form when it gives the `expected` text for every vector of that form in [normalization/vectors.tsv](normalization/vectors.tsv), or the same vectors as JSON in [normalization/vectors.json](normalization/vectors.json), and when normalizing its own output again changes nothing. The `rules` column names the rules each vector exercises; implementations need not count rules.

kannadi implements the profile in `kannadi::normalize` (canonical), `kannadi::normalize::normalize_as` and `normalize_counted` (any form, with counts per rule) and the command `kannadi normalize`. kannadi's legacy font converter writes canonical text: its tests check every golden fixture line, and random glyph sequences that convert cleanly.

## Changes

Changes to a rule change the profile's version: a new rule in the search form is a minor version, and any change to the canonical form is a major version. Propose changes as issues on [kannadi](https://github.com/openkannada/kannadi), with counts and rendering evidence.
