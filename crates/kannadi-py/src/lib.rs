//! Python bindings for kannadi. Offsets are Python string indices (code points).

use kannadi::legacy::{self, Layout, Options};
use kannadi::normalize::Form;
use kannadi::Scheme;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyString;

/// Code-point index of each byte offset in `text`, for offsets in increasing order.
fn indices(text: &str, offsets: impl IntoIterator<Item = usize>) -> Vec<usize> {
    let (mut byte, mut index) = (0, 0);
    offsets
        .into_iter()
        .map(|offset| {
            index += text[byte..offset].chars().count();
            byte = offset;
            index
        })
        .collect()
}

fn index(text: &str, offset: usize) -> usize {
    text[..offset].chars().count()
}

fn layout(name: &str) -> PyResult<Layout> {
    name.parse()
        .map_err(|e: legacy::UnknownLayout| PyValueError::new_err(e.to_string()))
}

fn scheme(name: &str) -> PyResult<Scheme> {
    name.parse()
        .map_err(|e: kannadi::UnknownScheme| PyValueError::new_err(e.to_string()))
}

/// The result of `to_unicode`.
#[pyclass(frozen, get_all, module = "kannadi")]
struct Conversion {
    /// The Unicode text. Unknown glyphs are kept as they were.
    text: String,
    /// `(index, glyph)` for each glyph that could not be placed; the index is in the input.
    unknown: Vec<(usize, char)>,
    /// `(start, end)` index ranges in `text` that are not valid syllables.
    malformed: Vec<(usize, usize)>,
}

#[pymethods]
impl Conversion {
    /// True if every glyph was converted and every syllable is well formed.
    #[getter]
    fn is_clean(&self) -> bool {
        self.unknown.is_empty() && self.malformed.is_empty()
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let text = PyString::new(py, &self.text).repr()?;
        let unknown = self.unknown.clone().into_pyobject(py)?.repr()?;
        let malformed = self.malformed.clone().into_pyobject(py)?.repr()?;
        Ok(format!(
            "Conversion(text={text}, unknown={unknown}, malformed={malformed})"
        ))
    }
}

/// The result of `from_unicode`.
#[pyclass(frozen, get_all, module = "kannadi")]
struct Encoding {
    /// The glyph codes as Windows-1252 characters, ready to set in a font with the layout.
    text: String,
    /// `(index, character)` for each input character with no glyph in the layout.
    unencodable: Vec<(usize, char)>,
    /// `(start, end)` index ranges in the input that are not valid aksharas.
    malformed: Vec<(usize, usize)>,
}

#[pymethods]
impl Encoding {
    /// True if every character was encoded and every akshara is well formed.
    #[getter]
    fn is_clean(&self) -> bool {
        self.unencodable.is_empty() && self.malformed.is_empty()
    }

    fn __repr__(&self, py: Python<'_>) -> PyResult<String> {
        let text = PyString::new(py, &self.text).repr()?;
        let unencodable = self.unencodable.clone().into_pyobject(py)?.repr()?;
        let malformed = self.malformed.clone().into_pyobject(py)?.repr()?;
        Ok(format!(
            "Encoding(text={text}, unencodable={unencodable}, malformed={malformed})"
        ))
    }
}

/// Converts text typed in a legacy font with the given layout to Unicode Kannada.
///
/// Layouts: "nudi" (also "baraha"), "akruti", "shree-lipi", "nudi-b" and "kan-eng".
#[pyfunction]
#[pyo3(signature = (text, layout = "nudi", *, kannada_digits = false, zero_as_anusvara = true))]
fn to_unicode(
    text: &str,
    layout: &str,
    kannada_digits: bool,
    zero_as_anusvara: bool,
) -> PyResult<Conversion> {
    let options = Options {
        kannada_digits,
        zero_as_anusvara,
    };
    let c = legacy::to_unicode(text, self::layout(layout)?, &options);
    let at = indices(text, c.unknown.iter().map(|u| u.offset));
    Ok(Conversion {
        unknown: at
            .into_iter()
            .zip(c.unknown.iter().map(|u| u.glyph))
            .collect(),
        malformed: c
            .malformed
            .iter()
            .map(|r| (index(&c.text, r.start), index(&c.text, r.end)))
            .collect(),
        text: c.text,
    })
}

/// Encodes Unicode Kannada as glyph codes of a legacy layout, such as "nudi".
#[pyfunction]
#[pyo3(signature = (text, layout = "nudi"))]
fn from_unicode(text: &str, layout: &str) -> PyResult<Encoding> {
    let e = legacy::from_unicode(text, self::layout(layout)?);
    let at = indices(text, e.unencodable.iter().map(|u| u.offset));
    Ok(Encoding {
        unencodable: at
            .into_iter()
            .zip(e.unencodable.iter().map(|u| u.character))
            .collect(),
        malformed: e
            .malformed
            .iter()
            .map(|r| (index(text, r.start), index(text, r.end)))
            .collect(),
        text: e.text,
    })
}

/// The layout that reads `text` most cleanly.
#[pyfunction]
fn best_layout(text: &str) -> &'static str {
    legacy::best_layout(text).name()
}

/// The layout of the font named `name`, such as "nudi" for BRH Kannada, or None.
#[pyfunction]
fn font_layout(name: &str) -> Option<&'static str> {
    legacy::font_layout(name).map(Layout::name)
}

/// True for a font that draws Kannada at the codes of Latin letters.
#[pyfunction]
fn is_legacy_font(name: &str) -> bool {
    legacy::is_legacy_font(name)
}

/// Text in a form of the Kannada text normalization profile: "canonical", "legacy" or "search".
#[pyfunction]
#[pyo3(signature = (text, form = "canonical"))]
fn normalize(text: &str, form: &str) -> PyResult<String> {
    let form = Form::from_name(form)
        .ok_or_else(|| PyValueError::new_err(format!("unknown form {form:?}")))?;
    Ok(kannadi::normalize::normalize_as(text, form))
}

/// Splits text into aksharas (orthographic syllables) and the characters between them.
#[pyfunction]
fn aksharas(text: &str) -> Vec<&str> {
    kannadi::split(text)
}

/// The number of aksharas in `text`.
#[pyfunction]
fn count_aksharas(text: &str) -> usize {
    kannadi::count_aksharas(text)
}

/// Converts text between "kannada", "iso15919", "iast" and "baraha".
#[pyfunction]
fn transliterate(text: &str, source: &str, target: &str) -> PyResult<String> {
    Ok(kannadi::transliterate(
        text,
        scheme(source)?,
        scheme(target)?,
    ))
}

/// Maps the Old Kannada letters ೞ, ಱ and ೝ to modern ones.
#[pyfunction]
fn modernize(text: &str) -> String {
    kannadi::modernize(text)
}

#[pymodule]
#[pyo3(name = "kannadi")]
fn kannadi_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    m.add_class::<Conversion>()?;
    m.add_class::<Encoding>()?;
    m.add_function(wrap_pyfunction!(to_unicode, m)?)?;
    m.add_function(wrap_pyfunction!(from_unicode, m)?)?;
    m.add_function(wrap_pyfunction!(best_layout, m)?)?;
    m.add_function(wrap_pyfunction!(font_layout, m)?)?;
    m.add_function(wrap_pyfunction!(is_legacy_font, m)?)?;
    m.add_function(wrap_pyfunction!(normalize, m)?)?;
    m.add_function(wrap_pyfunction!(aksharas, m)?)?;
    m.add_function(wrap_pyfunction!(count_aksharas, m)?)?;
    m.add_function(wrap_pyfunction!(transliterate, m)?)?;
    m.add_function(wrap_pyfunction!(modernize, m)?)?;
    Ok(())
}
