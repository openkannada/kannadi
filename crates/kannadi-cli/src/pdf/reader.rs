//! Walks a page's content and collects its text as lines of pieces, each piece in
//! 1 font.

use std::collections::HashMap;
use std::rc::Rc;

use anyhow::{Context, Result};
use kannadi::legacy::decode_windows_1252;
use lopdf::content::{Content, Operation};
use lopdf::{Dictionary, Document, Encoding, Object, Stream};

use crate::convert::Settings;

use super::cmap::CMap;
use super::encoding::{differences, glyph_codes};
use super::fonts::Fonts;
use super::widths::Widths;
use super::{resolve, LIMIT};

/// Forms may draw other forms; deeper nesting is taken as a loop.
const MAX_FORM_DEPTH: usize = 8;

/// The most operations 1 page may run, counting each time a form is drawn, so
/// forms that draw each other many times cannot hang the reader.
const MAX_OPERATIONS: usize = 1_000_000;

/// Text shown in 1 font, or a space the layout implies.
#[derive(Clone, Debug)]
pub enum Piece {
    Text {
        font: usize,
        /// The string bytes, or the encoding's reading of them.
        raw: String,
        /// The font's ToUnicode reading, where the font has a map of its own.
        mapped: Option<String>,
        /// For a font not named as legacy that has a ToUnicode map, and for
        /// every composite one: its text through the map, and how many glyph
        /// codes have no Unicode text.
        unicode: Option<(String, usize)>,
    },
    Space,
    /// A marked-content span with an `/ActualText` replacement, such as the
    /// spans Chrome writes for each conjunct, and the pieces drawn inside it.
    Actual {
        text: String,
        inner: Vec<Piece>,
    },
}

pub type Line = Vec<Piece>;

/// A page's text, and how many strings were drawn in fonts the page does not define.
pub struct Page {
    pub lines: Vec<Line>,
    pub lost: usize,
}

/// Reads 1 page whose content streams decompress to at most `limit` bytes.
pub fn read_page(
    doc: &Document,
    page: lopdf::ObjectId,
    fonts: &mut Fonts,
    settings: &Settings,
    limit: usize,
) -> Result<Page> {
    let content = doc
        .get_page_content_with_limit(page, limit)
        .context("reading the page")?;
    let (inline, inherited) = doc.get_page_resources(page).unwrap_or_default();
    let mut resources: Vec<&Dictionary> = inline.into_iter().collect();
    resources.extend(
        inherited
            .iter()
            .filter_map(|&id| doc.get_dictionary(id).ok()),
    );
    let mut reader = PageReader {
        doc,
        fonts,
        settings,
        page_fonts: HashMap::new(),
        forms: HashMap::new(),
        lines: vec![Vec::new()],
        pen: Pen::default(),
        current: None,
        last_string: None,
        saved: Vec::new(),
        lost: 0,
        operations_left: MAX_OPERATIONS,
        marked: Vec::new(),
        actual: None,
    };
    let content = Content::decode(&content).context("reading the page's content")?;
    reader.walk(&content.operations, &resources, 0)?;
    Ok(Page {
        lines: reader.lines,
        lost: reader.lost,
    })
}

struct Form<'d> {
    operations: Vec<Operation>,
    resources: Option<&'d Dictionary>,
    matrix: Matrix,
}

/// An affine transformation `[a b c d e f]`, as PDF writes it.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Matrix([f32; 6]);

impl Matrix {
    const IDENTITY: Matrix = Matrix([1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    fn from_operands(operands: &[Object]) -> Option<Matrix> {
        let numbers: Vec<f32> = operands.iter().filter_map(|o| o.as_float().ok()).collect();
        let values: [f32; 6] = numbers.try_into().ok()?;
        values
            .iter()
            .all(|v| v.is_finite())
            .then_some(Matrix(values))
    }

    /// This transformation followed by `outer`.
    fn then(self, outer: Matrix) -> Matrix {
        let [a, b, c, d, e, f] = self.0;
        let [p, q, r, s, t, u] = outer.0;
        Matrix([
            a * p + b * r,
            a * q + b * s,
            c * p + d * r,
            c * q + d * s,
            e * p + f * r + t,
            e * q + f * s + u,
        ])
    }

    fn x(self, x: f32, y: f32) -> f32 {
        self.0[0] * x + self.0[2] * y + self.0[4]
    }

    fn y(self, x: f32, y: f32) -> f32 {
        self.0[1] * x + self.0[3] * y + self.0[5]
    }

    /// How much the transformation scales lengths, on average.
    fn scale(self) -> f32 {
        let [a, b, c, d, ..] = self.0;
        (a * d - b * c).abs().sqrt()
    }
}

/// How a font's strings become text.
/// How a font not named as legacy gives Unicode text.
enum UnicodeReading {
    /// A composite font and its ToUnicode map, if any. With no map, its text is lost.
    Composite(Option<Rc<CMap>>),
    /// A 1-byte font's map. A code the map lacks is read by the encoding.
    Simple(Rc<CMap>),
}

struct PageFont<'d> {
    index: usize,
    /// 1-byte fonts named as legacy: their bytes are the glyph codes.
    raw_codes: bool,
    /// Characters by code: for other 1-byte fonts with a `Differences` list, and
    /// for legacy fonts whose list names glyphs by their code.
    table: Option<Vec<char>>,
    encoding: Option<Encoding<'d>>,
    cmap: Option<Rc<CMap>>,
    /// How a font not named as legacy gives Unicode text. lopdf drops ToUnicode
    /// entries that give more than 1 character, which Kannada conjuncts and
    /// vowel signs need, and reads Type 3 fonts by their glyph names.
    unicode: Option<UnicodeReading>,
    widths: Rc<Widths>,
}

impl PageFont<'_> {
    fn raw(&self, bytes: &[u8]) -> String {
        if let Some(table) = &self.table {
            return bytes.iter().map(|&b| table[usize::from(b)]).collect();
        }
        if self.raw_codes {
            return decode_windows_1252(bytes);
        }
        self.encoding
            .as_ref()
            .and_then(|e| Document::decode_text(e, bytes).ok())
            .unwrap_or_else(|| decode_windows_1252(bytes))
    }

    /// The Unicode reading of a string, with how many codes have no text.
    fn unicode(&self, bytes: &[u8]) -> Option<(String, usize)> {
        Some(match self.unicode.as_ref()? {
            UnicodeReading::Composite(Some(cmap)) => cmap.read(bytes),
            UnicodeReading::Composite(None) => (String::new(), bytes.len().div_ceil(2)),
            UnicodeReading::Simple(cmap) => {
                let text = bytes
                    .iter()
                    .map(|b| {
                        let code = std::slice::from_ref(b);
                        cmap.get(code)
                            .map_or_else(|| self.raw(code), str::to_string)
                    })
                    .collect();
                (text, 0)
            }
        })
    }
}

/// Where the next glyph goes, for finding line breaks and word gaps without the
/// full text and graphics matrices.
struct Pen {
    size: f32,
    scale: f32,
    line_x: f32,
    x: f32,
    y: f32,
    /// The text leading, which `T*` moves down by.
    leading: f32,
    /// Where the last string ended, within this text object and line.
    end: Option<f32>,
    /// The current transformation matrix, from user space to the page.
    ctm: Matrix,
    /// The baseline of the last string shown, on the page.
    last_y: Option<f32>,
}

impl Default for Pen {
    fn default() -> Self {
        Pen {
            size: 0.0,
            scale: 1.0,
            line_x: 0.0,
            x: 0.0,
            y: 0.0,
            leading: 0.0,
            end: None,
            ctm: Matrix::IDENTITY,
            last_y: None,
        }
    }
}

impl Pen {
    fn em(&self) -> f32 {
        self.size.abs() * self.scale
    }

    /// `BT` starts with the identity text matrix. Gaps are measured within a text
    /// object only: glyph widths in real PDFs are not exact enough to measure
    /// across them.
    fn begin(&mut self) {
        self.line_x = 0.0;
        self.x = 0.0;
        self.y = 0.0;
        self.scale = 1.0;
        self.end = None;
    }

    fn move_by(&mut self, tx: f32, ty: f32) {
        self.line_x += tx * self.scale;
        self.x = self.line_x;
        self.y += ty * self.scale;
    }

    fn set(&mut self, scale: f32, x: f32, y: f32) {
        self.scale = if scale.abs() > 0.0 { scale.abs() } else { 1.0 };
        self.line_x = x;
        self.x = x;
        self.y = y;
    }

    /// Whether the next string starts a new line: its baseline is more than half
    /// the font size from the last one's, since vowel signs and subscripts are
    /// often drawn a little off the baseline.
    fn starts_line(&self) -> bool {
        let limit = (0.5 * self.em() * self.ctm.scale()).max(0.5);
        self.last_y
            .is_some_and(|y| (self.page_y() - y).abs() > limit)
    }

    fn page_y(&self) -> f32 {
        self.ctm.y(self.x, self.y)
    }

    /// Where the next string starts on the page, and the font size there.
    fn page_point(&self) -> (f32, f32, f32) {
        (
            self.ctm.x(self.x, self.y),
            self.page_y(),
            self.em() * self.ctm.scale(),
        )
    }

    /// Whether a string starting here leaves a word gap after the last one.
    fn gap(&self) -> bool {
        self.end.is_some_and(|end| self.x - end > 0.2 * self.em())
    }

    fn advance(&mut self, thousandths: f32) {
        self.x += thousandths / 1000.0 * self.em();
        self.end = Some(self.x);
        self.last_y = Some(self.page_y());
    }
}

struct PageReader<'d, 'f> {
    doc: &'d Document,
    fonts: &'f mut Fonts,
    settings: &'f Settings,
    /// By the address of the font's dictionary, since forms may reuse a name.
    page_fonts: HashMap<usize, PageFont<'d>>,
    /// Decoded forms by the address of their stream; `None` for a stream that is
    /// not a readable form.
    forms: HashMap<usize, Option<Rc<Form<'d>>>>,
    lines: Vec<Line>,
    pen: Pen,
    current: Option<&'d Dictionary>,
    /// The last string shown: its font, bytes and start on the page.
    last_string: Option<(usize, Vec<u8>, f32, f32)>,
    /// Fonts, sizes, leadings and transformations saved by `q`, restored by `Q`.
    saved: Vec<(Option<&'d Dictionary>, f32, f32, Matrix)>,
    lost: usize,
    operations_left: usize,
    /// Open marked-content sequences, each true if it began an `/ActualText`
    /// span that is being collected.
    marked: Vec<bool>,
    /// The open `/ActualText` span: its text, and where its pieces start.
    actual: Option<(String, usize, usize)>,
}

impl<'d> PageReader<'d, '_> {
    /// Reads a content stream drawn with `resources`, and the forms it draws.
    fn walk(
        &mut self,
        operations: &[Operation],
        resources: &[&'d Dictionary],
        depth: usize,
    ) -> Result<()> {
        self.operations_left = self
            .operations_left
            .checked_sub(operations.len())
            .context("the page draws too many operations; its forms may draw each other")?;
        for op in operations {
            let number = |i: usize| op.operands.get(i).and_then(|o| o.as_float().ok());
            let name = || op.operands.first().and_then(|o| o.as_name().ok());
            match op.operator.as_str() {
                "BT" => self.pen.begin(),
                "Tf" => {
                    self.current = name().and_then(|n| self.resource(resources, b"Font", n));
                    self.pen.size = number(1).unwrap_or(self.pen.size);
                }
                "Td" | "TD" => {
                    let ty = number(1).unwrap_or(0.0);
                    if op.operator == "TD" {
                        self.pen.leading = -ty;
                    }
                    self.pen.move_by(number(0).unwrap_or(0.0), ty);
                }
                "TL" => self.pen.leading = number(0).unwrap_or(self.pen.leading),
                "Tm" => {
                    let (a, e, f) = (number(0), number(4), number(5));
                    self.pen
                        .set(a.unwrap_or(1.0), e.unwrap_or(0.0), f.unwrap_or(0.0));
                }
                // Acrobat Distiller moves to the next part of a line with a
                // leading of 0, which stays on the baseline.
                "T*" | "'" | "\"" => self.pen.move_by(0.0, -self.pen.leading),
                "q" => self.saved.push(self.state()),
                "Q" => {
                    if let Some(state) = self.saved.pop() {
                        self.restore(state);
                    }
                }
                "cm" => {
                    if let Some(m) = Matrix::from_operands(&op.operands) {
                        self.pen.ctm = m.then(self.pen.ctm);
                    }
                }
                "Do" if depth < MAX_FORM_DEPTH => {
                    if let Some(form) = name().and_then(|n| self.form(resources, n)) {
                        let mut inner: Vec<&Dictionary> = form.resources.into_iter().collect();
                        inner.extend_from_slice(resources);
                        // A form is drawn inside its own `q` and `Q`.
                        let state = self.state();
                        let saved = std::mem::take(&mut self.saved);
                        self.pen.ctm = form.matrix.then(state.3);
                        let walked = self.walk(&form.operations, &inner, depth + 1);
                        self.restore(state);
                        self.saved = saved;
                        walked?;
                    }
                }
                "BMC" => self.marked.push(false),
                "BDC" => self.begin_marked(op),
                "EMC" => self.end_marked(),
                "Tj" | "TJ" => {}
                _ => continue,
            }
            if matches!(op.operator.as_str(), "Tj" | "TJ" | "'" | "\"") {
                self.show(op);
            }
        }
        Ok(())
    }

    /// The part of the graphics state the reader follows, as `q` saves it.
    fn state(&self) -> (Option<&'d Dictionary>, f32, f32, Matrix) {
        (self.current, self.pen.size, self.pen.leading, self.pen.ctm)
    }

    fn restore(&mut self, (font, size, leading, ctm): (Option<&'d Dictionary>, f32, f32, Matrix)) {
        self.current = font;
        self.pen.size = size;
        self.pen.leading = leading;
        self.pen.ctm = ctm;
    }

    /// Starts collecting an `/ActualText` span, unless one is already open.
    fn begin_marked(&mut self, op: &Operation) {
        let text = op
            .operands
            .get(1)
            .and_then(|o| o.as_dict().ok())
            .and_then(|d| d.get(b"ActualText").ok())
            .and_then(|o| o.as_str().ok())
            .map(text_string);
        let starts = text.is_some() && self.actual.is_none();
        if let (true, Some(text)) = (starts, text) {
            let line = self.lines.len() - 1;
            let at = self.lines[line].len();
            self.actual = Some((text, line, at));
        }
        self.marked.push(starts);
    }

    /// Ends a marked-content sequence. An `/ActualText` span that stayed on 1
    /// line becomes 1 piece; one that broke the line keeps its pieces as drawn.
    fn end_marked(&mut self) {
        if self.marked.pop() != Some(true) {
            return;
        }
        let Some((text, line, at)) = self.actual.take() else {
            return;
        };
        if line + 1 != self.lines.len() {
            return;
        }
        let inner: Vec<Piece> = self.lines[line].drain(at..).collect();
        self.lines[line].push(Piece::Actual { text, inner });
    }

    fn break_line(&mut self) {
        if self.lines.last().is_some_and(|l| !l.is_empty()) {
            self.lines.push(Vec::new());
            // A span that opened before its first glyph moved to a new line
            // starts on that line.
            if let Some((_, line, at)) = &mut self.actual {
                if *at == self.lines[*line].len() {
                    *line = self.lines.len() - 1;
                    *at = 0;
                }
            }
        }
        self.pen.end = None;
    }

    /// A named entry in a resource category, such as a font, from the innermost
    /// resources that have it.
    fn resource(
        &self,
        resources: &[&'d Dictionary],
        category: &[u8],
        name: &[u8],
    ) -> Option<&'d Dictionary> {
        resources.iter().find_map(|r| {
            let entries = resolve(self.doc, r.get(category).ok()?)?.as_dict().ok()?;
            resolve(self.doc, entries.get(name).ok()?)?.as_dict().ok()
        })
    }

    /// A form XObject by its name in `resources`, decoded once per page.
    fn form(&mut self, resources: &[&'d Dictionary], name: &[u8]) -> Option<Rc<Form<'d>>> {
        let doc = self.doc;
        let stream = resources.iter().find_map(|r| {
            let entries = resolve(doc, r.get(b"XObject").ok()?)?.as_dict().ok()?;
            resolve(doc, entries.get(name).ok()?)?.as_stream().ok()
        })?;
        let key = std::ptr::from_ref(stream) as usize;
        self.forms
            .entry(key)
            .or_insert_with(|| decode_form(doc, stream).map(Rc::new))
            .clone()
    }

    fn page_font(&mut self, dict: &'d Dictionary) -> &PageFont<'d> {
        let key = std::ptr::from_ref(dict) as usize;
        let (doc, settings) = (self.doc, self.settings);
        let fonts = &mut *self.fonts;
        self.page_fonts.entry(key).or_insert_with(|| {
            let simple = dict
                .get(b"Subtype")
                .and_then(Object::as_name)
                .is_ok_and(|s| s != b"Type0");
            let index = fonts.index(doc, dict, settings);
            let named = fonts.is_named_legacy(index);
            let raw_codes = simple && named;
            let encoding = dict.get_font_encoding_with_limit(doc, LIMIT).ok();
            let by_lopdf = |b: u8| {
                let text = Document::decode_text(encoding.as_ref()?, &[b]).ok()?;
                let mut chars = text.chars();
                chars.next().filter(|_| chars.next().is_none())
            };
            PageFont {
                index,
                raw_codes,
                table: if raw_codes {
                    glyph_codes(doc, dict)
                } else if simple {
                    differences(doc, dict, by_lopdf)
                } else {
                    None
                },
                encoding,
                cmap: if named { fonts.cmap(doc, dict) } else { None },
                unicode: if named {
                    None
                } else if simple {
                    fonts.cmap(doc, dict).map(UnicodeReading::Simple)
                } else {
                    Some(UnicodeReading::Composite(fonts.cmap(doc, dict)))
                },
                widths: fonts.widths(doc, dict),
            }
        })
    }

    fn show(&mut self, op: &Operation) {
        let Some(dict) = self.current else {
            self.lost += 1;
            return;
        };
        if self.pen.starts_line() {
            self.break_line();
        }
        let (index, widths) = {
            let font = self.page_font(dict);
            (font.index, Rc::clone(&font.widths))
        };
        for item in shown(op) {
            match item {
                Shown::Adjust(n) => {
                    if n < -200.0 {
                        self.push(Piece::Space, index);
                    }
                    self.pen.advance(-n);
                }
                Shown::Bytes(bytes) => {
                    if self.overprints(index, bytes) {
                        self.pen.advance(widths.advance(bytes));
                        continue;
                    }
                    if self.pen.gap() {
                        self.push(Piece::Space, index);
                    }
                    let font = self.page_font(dict);
                    let raw = font.raw(bytes);
                    let mapped = font.cmap.as_ref().map(|m| m.decode(bytes));
                    let unicode = font.unicode(bytes);
                    self.fonts.add_text(index, &raw, mapped.as_deref());
                    self.push(
                        Piece::Text {
                            font: index,
                            raw,
                            mapped,
                            unicode,
                        },
                        index,
                    );
                    self.pen.advance(widths.advance(bytes));
                }
            }
        }
    }

    /// Whether `bytes` repeat the last string in the same font, starting within a
    /// tenth of the font size of it: the copies some PDF makers draw, a little
    /// apart, to make text look bold.
    fn overprints(&mut self, font: usize, bytes: &[u8]) -> bool {
        let (x, y, em) = self.pen.page_point();
        let repeat = self.last_string.as_ref().is_some_and(|(f, b, px, py)| {
            *f == font && b == bytes && (x - px).abs().max((y - py).abs()) < 0.1 * em
        });
        if !repeat && !bytes.is_empty() {
            self.last_string = Some((font, bytes.to_vec(), x, y));
        }
        repeat
    }

    fn push(&mut self, piece: Piece, font: usize) {
        if matches!(piece, Piece::Space) {
            self.fonts.add_text(font, " ", Some(" "));
        }
        if let Some(line) = self.lines.last_mut() {
            line.push(piece);
        }
    }
}

/// A PDF text string: UTF-16BE or UTF-8 after a byte order mark, or else
/// PDFDocEncoding, read here as Latin-1.
fn text_string(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let units: Vec<u16> = rest
            .as_chunks::<2>()
            .0
            .iter()
            .map(|p| u16::from_be_bytes(*p))
            .collect();
        String::from_utf16_lossy(&units)
    } else if let Some(rest) = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(rest).into_owned()
    } else {
        bytes.iter().map(|&b| char::from(b)).collect()
    }
}

fn decode_form<'d>(doc: &'d Document, stream: &'d Stream) -> Option<Form<'d>> {
    if stream.dict.get(b"Subtype").and_then(Object::as_name).ok() != Some(b"Form".as_slice()) {
        return None;
    }
    let entry = |key: &[u8]| stream.dict.get(key).ok().and_then(|o| resolve(doc, o));
    let content = stream.decompressed_content_with_limit(LIMIT).ok()?;
    Some(Form {
        operations: Content::decode(&content).ok()?.operations,
        resources: entry(b"Resources").and_then(|o| o.as_dict().ok()),
        matrix: entry(b"Matrix")
            .and_then(|o| o.as_array().ok())
            .and_then(|a| Matrix::from_operands(a))
            .unwrap_or(Matrix::IDENTITY),
    })
}

enum Shown<'a> {
    Bytes(&'a [u8]),
    /// A `TJ` position adjustment, in thousandths of the font size; negative
    /// numbers move right.
    Adjust(f32),
}

fn shown(op: &Operation) -> Vec<Shown<'_>> {
    let mut items = Vec::new();
    for operand in &op.operands {
        match operand {
            Object::String(bytes, _) => items.push(Shown::Bytes(bytes)),
            Object::Array(array) => {
                for o in array {
                    match o {
                        Object::String(bytes, _) => items.push(Shown::Bytes(bytes)),
                        o => items.extend(o.as_float().ok().map(Shown::Adjust)),
                    }
                }
            }
            _ => {}
        }
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_strings_read_in_each_encoding() {
        assert_eq!(text_string(b"\xFE\xFF\x0C\x95\x0C\xBE"), "ಕಾ");
        assert_eq!(text_string("\u{FEFF}ಕಾ".as_bytes()), "ಕಾ");
        assert_eq!(text_string(b"caf\xE9"), "café");
    }
}
