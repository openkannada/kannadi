//! WebAssembly bindings for [`kannadi`], used by the converter page in `web/`.
//!
//! Offsets and ranges are in UTF-16 code units, the unit JavaScript strings use,
//! so they work with `String.prototype.slice` and `setSelectionRange` as given.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

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
    let malformed = converted
        .malformed
        .iter()
        .flat_map(|r| [output.at(r.start), output.at(r.end)])
        .collect();

    Conversion {
        text: converted.text,
        unknown,
        malformed,
    }
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
