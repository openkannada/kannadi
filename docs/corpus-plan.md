# Legacy test corpus plan

Status: the first tier 2 set is in [corpus/](../corpus/README.md): 207 lines from 31 Karnataka government pages of 2002. It keeps 1 line per row in a TSV, with the source URL on each row, in place of the per-document folders below, because every source is a short excerpt. Tier 1 is not started.

The legacy converter is only as good as its test documents. This plan comes before any converter code.

## What each fixture holds

```text
corpus/<id>/
  source.txt | source.docx | source.html   the legacy document, or an excerpt
  expected.txt                             the verified Unicode text
  meta.toml                                where it came from and who checked it
```

`meta.toml` fields:

| Field | Example |
|---|---|
| `source` | URL, or "typed for this corpus" |
| `retrieved` | 2026-11-20 |
| `license_basis` | "Official Gazette, Indian Copyright Act s. 52(1)(q)", "contributed under CC0", "typed for this corpus" |
| `font` | Font name exactly as it appears in the file |
| `encoding` | nudi or baraha |
| `verified_by` | Name of the Kannada reader who checked `expected.txt` |
| `verified_on` | Date |
| `notes` | Anything odd: hand-fixed glyphs, damaged text |

## 3 tiers

### Tier 1: synthetic coverage set (first)

Typed in Nudi and in Baraha legacy mode on Windows, from a fixed Unicode script, and saved as TXT and DOCX. Because the Unicode source exists first, `expected.txt` is known exactly.

The script covers:

- the full kagunita: 36 consonants with all 15 vowel signs, 540 aksharas;
- every ottu form, under consonants with and without vowel signs;
- arkavattu with and without vowel signs and ottus (ಅರ್ಕ, ಕಾರ್ಯ, ಸೂರ್ಯ, ಕಾರ್ತಿಕ);
- the special conjuncts ಜ್ಞ, ಕ್ಷ, ಶ್ರೀ and 3-consonant clusters (ಸ್ತ್ರೀ, ರಾಷ್ಟ್ರ);
- known hard cases: ವಿಜ್ಞಾನ, ಪುಸ್ತಕ, ಮೊ, ಮೋ, ZWJ in ಗ್ರಿಡ್‍ಗೆ, ottu at the end of a word;
- digits, punctuation, and English words in the same paragraph.

Effort: about 2 hours of typing per encoding by someone with Nudi and Baraha installed, plus a check. Bar: zero differences.

### Tier 2: real public documents

Real documents show which fonts, versions and habits occur in practice. Targets: at least 30 documents from at least 5 sources, spread over at least 3 different years.

Candidate sources, all public:

- Karnataka Gazette notifications and government orders published as DOC or DOCX;
- department circulars and tender notices on Karnataka government websites;
- university and board question papers and syllabi;
- older Kannada web pages that still use `<font face="Nudi...">` or Baraha fonts (via the Internet Archive).

Copyright: Official Gazette material may be reproduced under section 52(1)(q) of the Indian Copyright Act. For anything else, the repository keeps only a short excerpt, or keeps the file out of Git and records its URL and SHA-256 so the test can download it. No document goes in without a recorded basis.

`expected.txt` for tier 2: run the draft converter, then a Kannada reader compares the output with the document rendered in its own font, side by side, and fixes every difference by hand. The fixed text is the expected output. Bar: akshara error rate below 0.1% per document, with every remaining error filed as an issue.

### Tier 3: archives with permission

Newspaper and publisher archives are the largest real-world use, and many are in Shree-Lipi. Approach publishers once the converter passes tiers 1 and 2, and add documents only with written permission.

## Help that would speed this up

**Kannada readers** to verify tier 2. A short guide and a side-by-side review page would let volunteers help.

## Order of work

This was the plan on 15 November 2026. What was built is in [corpus/](../corpus/README.md): real government pages came first, and the typed script of tier 1 was not made.

1. Write the tier 1 Unicode script (this repository, `corpus/script.txt`).
2. Get it typed in Nudi and Baraha; commit the files with `expected.txt`.
3. Write the font tables and the converter against tier 1 until it is clean.
4. Collect tier 2 documents and verify them.
5. Report the akshara error rate per document in CI.
