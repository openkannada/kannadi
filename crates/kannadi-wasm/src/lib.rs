//! WebAssembly bindings for [`kannadi`], used by the editor page in `web/`.
//!
//! Offsets and ranges are in UTF-16 code units, the unit JavaScript strings use,
//! so they work with `String.prototype.slice` and `setSelectionRange` as given.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod fix;

use kannadi::legacy;
use kannadi::Scheme;
use wasm_bindgen::prelude::*;

/// Settings for [`nudi_to_unicode`]. The defaults match [`legacy::Options`].
#[wasm_bindgen(js_name = NudiOptions)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NudiOptions {
    /// Write digits as Kannada digits, for text set in the `k` variants of the Nudi fonts.
    #[wasm_bindgen(js_name = kannadaDigits)]
    pub kannada_digits: bool,
    /// Turn a zero typed in place of an anusvara into ಂ (U+0C82).
    #[wasm_bindgen(js_name = zeroAsAnusvara)]
    pub zero_as_anusvara: bool,
}

#[wasm_bindgen(js_class = NudiOptions)]
impl NudiOptions {
    /// The default settings.
    #[wasm_bindgen(constructor)]
    pub fn new() -> NudiOptions {
        legacy::Options::default().into()
    }
}

impl Default for NudiOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl From<legacy::Options> for NudiOptions {
    fn from(o: legacy::Options) -> Self {
        NudiOptions {
            kannada_digits: o.kannada_digits,
            zero_as_anusvara: o.zero_as_anusvara,
        }
    }
}

impl From<NudiOptions> for legacy::Options {
    fn from(o: NudiOptions) -> Self {
        legacy::Options {
            kannada_digits: o.kannada_digits,
            zero_as_anusvara: o.zero_as_anusvara,
        }
    }
}

/// The result of [`nudi_to_unicode`]. The reports are flat arrays, so a large
/// input crosses into JavaScript as 2 typed arrays.
#[wasm_bindgen]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conversion {
    text: String,
    unknown: Vec<u32>,
    unknown_in_text: Vec<u32>,
    malformed: Vec<u32>,
}

#[wasm_bindgen]
impl Conversion {
    /// The Unicode text. Unknown glyphs are kept as they were.
    #[wasm_bindgen(getter)]
    pub fn text(&self) -> String {
        self.text.clone()
    }

    /// Offsets in the input of glyphs the converter could not place, ascending.
    #[wasm_bindgen(getter)]
    pub fn unknown(&self) -> Vec<u32> {
        self.unknown.clone()
    }

    /// Offsets in the output text of the same glyphs, which the converter keeps
    /// as they were. Same order as [`Conversion::unknown`].
    #[wasm_bindgen(getter, js_name = unknownInText)]
    pub fn unknown_in_text(&self) -> Vec<u32> {
        self.unknown_in_text.clone()
    }

    /// Ranges of the output text that are not valid syllables, as
    /// `[start0, end0, start1, end1, ...]`, ascending.
    #[wasm_bindgen(getter)]
    pub fn malformed(&self) -> Vec<u32> {
        self.malformed.clone()
    }

    /// True if every glyph was converted and every syllable is well formed.
    #[wasm_bindgen(getter, js_name = isClean)]
    pub fn is_clean(&self) -> bool {
        self.unknown.is_empty() && self.malformed.is_empty()
    }
}

/// Converts text typed in a Nudi or Baraha ANSI font to Unicode Kannada.
/// See [`legacy::nudi_to_unicode`].
#[wasm_bindgen(js_name = nudiToUnicode)]
pub fn nudi_to_unicode(text: &str, options: &NudiOptions) -> Conversion {
    let converted = legacy::nudi_to_unicode(text, &(*options).into());

    let mut input = Utf16Cursor::new(text);
    let unknown = converted
        .unknown
        .iter()
        .map(|u| input.at(u.offset))
        .collect();

    let mut output = Utf16Cursor::new(&converted.text);
    let mut from = 0;
    let unknown_in_text = converted
        .unknown
        .iter()
        .filter_map(|u| {
            let at = from + converted.text[from..].find(u.glyph)?;
            from = at + u.glyph.len_utf8();
            Some(output.at(at))
        })
        .collect();

    let mut output = Utf16Cursor::new(&converted.text);
    let malformed = converted
        .malformed
        .iter()
        .flat_map(|r| [output.at(r.start), output.at(r.end)])
        .collect();

    Conversion {
        text: converted.text,
        unknown,
        unknown_in_text,
        malformed,
    }
}

/// Ranges of `text` that are not well-formed aksharas, as
/// `[start0, end0, start1, end1, ...]`, ascending. See
/// [`kannadi::Akshara::is_well_formed`].
#[wasm_bindgen]
pub fn malformed(text: &str) -> Vec<u32> {
    let mut cursor = Utf16Cursor::new(text);
    let mut out = Vec::new();
    let mut pos = 0;
    for a in kannadi::aksharas(text) {
        if !a.is_well_formed() {
            out.push(cursor.at(pos));
            out.push(cursor.at(pos + a.text.len()));
        }
        pos += a.text.len();
    }
    out
}

/// A repair for a malformed akshara: replace `start..end` with `replacement`.
#[wasm_bindgen]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fix {
    /// First code unit to replace.
    pub start: u32,
    /// Code unit after the last one to replace.
    pub end: u32,
    replacement: String,
    kind: &'static str,
}

#[wasm_bindgen]
impl Fix {
    /// The text to put in place of the range.
    #[wasm_bindgen(getter)]
    pub fn replacement(&self) -> String {
        self.replacement.clone()
    }

    /// `normalize`, `join`, `remove`, `keep-first` or `keep-last`.
    #[wasm_bindgen(getter)]
    pub fn kind(&self) -> String {
        self.kind.to_string()
    }
}

/// Up to 4 checked repairs for the malformed akshara at `start..end`, most
/// likely first. Empty when the range is not exactly 1 malformed akshara.
#[wasm_bindgen]
pub fn fixes(text: &str, start: u32, end: u32) -> Vec<Fix> {
    let (Some(start), Some(end)) = (byte_offset(text, start), byte_offset(text, end)) else {
        return Vec::new();
    };
    if start > end {
        return Vec::new();
    }
    fix::fixes(text, start..end)
        .into_iter()
        .map(|f| {
            let mut cursor = Utf16Cursor::new(text);
            Fix {
                start: cursor.at(f.range.start),
                end: cursor.at(f.range.end),
                replacement: f.replacement,
                kind: f.kind.name(),
            }
        })
        .collect()
}

/// Counts vowel and consonant aksharas. See [`kannadi::count_aksharas`].
#[wasm_bindgen(js_name = countAksharas)]
pub fn count_aksharas(text: &str) -> u32 {
    u32::try_from(kannadi::count_aksharas(text)).unwrap_or(u32::MAX)
}

/// Maps Old Kannada letters to modern ones. See [`kannadi::modernize()`].
#[wasm_bindgen]
pub fn modernize(text: &str) -> String {
    kannadi::modernize(text)
}

/// The byte offset of UTF-16 offset `unit`, if it falls on a char boundary.
fn byte_offset(text: &str, unit: u32) -> Option<usize> {
    let unit = usize::try_from(unit).ok()?;
    let mut units = 0;
    for (byte, c) in text.char_indices() {
        if units == unit {
            return Some(byte);
        }
        if units > unit {
            return None;
        }
        units += c.len_utf16();
    }
    (units == unit).then_some(text.len())
}

/// Decodes the bytes of a legacy text file. See [`legacy::decode_windows_1252`].
#[wasm_bindgen(js_name = decodeWindows1252)]
pub fn decode_windows_1252(bytes: &[u8]) -> String {
    legacy::decode_windows_1252(bytes)
}

/// Converts `text` between `kannada`, `iso15919`, `iast` and `baraha`.
/// See [`kannadi::transliterate()`].
///
/// # Errors
///
/// Throws if a scheme name is not recognized.
#[wasm_bindgen]
pub fn transliterate(text: &str, from: &str, to: &str) -> Result<String, JsError> {
    let from: Scheme = from.parse()?;
    let to: Scheme = to.parse()?;
    Ok(kannadi::transliterate(text, from, to))
}

/// Makes equal-looking Kannada text compare equal. See [`kannadi::normalize()`].
#[wasm_bindgen]
pub fn normalize(text: &str) -> String {
    kannadi::normalize(text)
}

/// Splits text into aksharas and other segments. See [`kannadi::aksharas()`].
#[wasm_bindgen]
pub fn aksharas(text: &str) -> Vec<String> {
    kannadi::aksharas(text)
        .map(|a| a.text.to_string())
        .collect()
}

/// Maps byte offsets in a string to UTF-16 offsets. Ascending queries cost
/// 1 pass over the string in total.
struct Utf16Cursor<'a> {
    text: &'a str,
    byte: usize,
    unit: usize,
}

impl<'a> Utf16Cursor<'a> {
    fn new(text: &'a str) -> Self {
        Utf16Cursor {
            text,
            byte: 0,
            unit: 0,
        }
    }

    /// `byte` must be a char boundary of the text.
    fn at(&mut self, byte: usize) -> u32 {
        if byte < self.byte {
            self.byte = 0;
            self.unit = 0;
        }
        self.unit += self.text[self.byte..byte].encode_utf16().count();
        self.byte = byte;
        u32::try_from(self.unit).unwrap_or(u32::MAX)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_counts_utf16_units() {
        // ಕ is 3 bytes and 1 unit; 𝔸 (U+1D538) is 4 bytes and 2 units.
        let text = "ಕa𝔸b";
        let mut c = Utf16Cursor::new(text);
        assert_eq!(c.at(0), 0);
        assert_eq!(c.at(3), 1);
        assert_eq!(c.at(4), 2);
        assert_eq!(c.at(8), 4);
        assert_eq!(c.at(9), 5);
        assert_eq!(c.at(3), 1, "a query before the last one starts over");
    }

    #[test]
    fn converts_clean_text() {
        let c = nudi_to_unicode("PÀ£ÀßqÀ", &NudiOptions::new());
        assert_eq!(c.text(), "ಕನ್ನಡ");
        assert!(c.is_clean());
    }

    #[test]
    fn options_reach_the_converter() {
        let mut options = NudiOptions::new();
        assert_eq!(legacy::Options::from(options), legacy::Options::default());
        options.kannada_digits = true;
        assert_eq!(nudi_to_unicode("1956", &options).text(), "೧೯೫೬");
        options.kannada_digits = false;
        assert_eq!(nudi_to_unicode("1956", &options).text(), "1956");
    }

    #[test]
    fn report_offsets_are_utf16() {
        let input = "PÀ£ÀßqÀ õ ¥Àæ";
        let c = nudi_to_unicode(input, &NudiOptions::new());
        let units: Vec<u16> = input.encode_utf16().collect();
        assert_eq!(c.unknown().len(), 1);
        assert_eq!(units[c.unknown()[0] as usize], 'õ' as u16);
        assert!(c.malformed().is_empty());
    }

    #[test]
    fn malformed_spans_select_the_bad_text() {
        // A vowel sign with no consonant before it, after 1 good syllable.
        let c = nudi_to_unicode("PÀ ÁA", &NudiOptions::new());
        let m = c.malformed();
        assert!(m.len() >= 2 && m.len().is_multiple_of(2), "{c:?}");
        let units: Vec<u16> = c.text().encode_utf16().collect();
        let bad = String::from_utf16(&units[m[0] as usize..m[1] as usize]).unwrap();
        let rust = legacy::nudi_to_unicode("PÀ ÁA", &legacy::Options::default());
        assert_eq!(bad, rust.text[rust.malformed[0].clone()]);
        assert!(m[0] > 0);
    }

    #[test]
    fn unknown_glyphs_are_found_in_the_output() {
        let c = nudi_to_unicode("PÀ£ÀßqÀ õ ¥Àæ õ", &NudiOptions::new());
        let units: Vec<u16> = c.text().encode_utf16().collect();
        assert_eq!(c.unknown_in_text().len(), 2);
        for at in c.unknown_in_text() {
            assert_eq!(units[at as usize], 'õ' as u16);
        }
        assert!(c.unknown_in_text()[0] < c.unknown_in_text()[1]);
    }

    #[test]
    fn malformed_ranges_are_utf16() {
        let text = "𝔸 ಕನ್ನಡ ಅಾ ಮನ ೆ";
        let m = malformed(text);
        let units: Vec<u16> = text.encode_utf16().collect();
        let parts: Vec<String> = m
            .chunks(2)
            .map(|r| String::from_utf16(&units[r[0] as usize..r[1] as usize]).unwrap())
            .collect();
        assert_eq!(parts, ["ಅಾ", " ೆ"]);
    }

    #[test]
    fn unicode_check_agrees_with_the_converter_on_real_text() {
        // RDPR page, 2002: both checks flag the virama after the unknown õ.
        // https://web.archive.org/web/20021218215751/http://www.kar.nic.in:80/rdpr/kan-www/aboutrdpr.html
        let c = nudi_to_unicode("gÁdQÃAiÀÄ ¥ÀæYõï ¨É¼É¸ÀÄªÀ", &NudiOptions::new());
        let (glyphs, unicode) = (c.malformed(), malformed(&c.text()));
        assert_eq!((glyphs.len(), unicode.len()), (2, 2));
        assert!(unicode[0] <= glyphs[0] && glyphs[1] <= unicode[1]);
    }

    #[test]
    fn fixes_use_utf16_offsets() {
        let text = "𝔸 ಮನ ೆ";
        let m = malformed(text);
        let f = fixes(text, m[0], m[1]);
        assert_eq!(f[0].kind(), "join");
        let mut units: Vec<u16> = text.encode_utf16().collect();
        let replacement: Vec<u16> = f[0].replacement().encode_utf16().collect();
        units.splice(f[0].start as usize..f[0].end as usize, replacement);
        assert_eq!(String::from_utf16(&units).unwrap(), "𝔸 ಮನೆ");
        assert!(fixes(text, 1, 2).is_empty(), "inside a surrogate pair");
        assert!(fixes(text, 99, 100).is_empty());
    }

    #[test]
    fn counts_and_modernizes() {
        assert_eq!(count_aksharas("ಕನ್ನಡ ನಾಡು, 2026"), 5);
        assert_eq!(modernize("ಬಾೞ್"), "ಬಾಳ್");
    }

    #[test]
    fn splits_aksharas() {
        assert_eq!(aksharas("ಕನ್ನಡ"), ["ಕ", "ನ್ನ", "ಡ"]);
    }

    #[test]
    fn transliterates_by_scheme_name() {
        assert_eq!(
            transliterate("ಕನ್ನಡ", "kannada", "iso15919").ok(),
            Some("kannaḍa".to_string())
        );
    }

    #[test]
    fn normalizes() {
        assert_eq!(normalize("ಕಿ\u{0CD5}"), "ಕೀ");
    }

    #[test]
    fn decodes_windows_1252() {
        assert_eq!(decode_windows_1252(&[0x50, 0xC0, 0x86]), "PÀ†");
    }
}
