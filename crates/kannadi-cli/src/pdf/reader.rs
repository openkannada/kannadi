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
#[derive(Debug)]
pub enum Piece {
    Text {
        font: usize,
        /// The string bytes, or the encoding's reading of them.
        raw: String,
        /// The font's ToUnicode reading, where the font has a map of its own.
        mapped: Option<String>,
    },
    Space,
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
struct PageFont<'d> {
    index: usize,
    /// 1-byte fonts named as legacy: their bytes are the glyph codes.
    raw_codes: bool,
    /// Characters by code: for other 1-byte fonts with a `Differences` list, and
    /// for legacy fonts whose list names glyphs by their code.
    table: Option<Vec<char>>,
    encoding: Option<Encoding<'d>>,
    cmap: Option<Rc<CMap>>,
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
}

/// Where the next glyph goes, for finding line breaks and word gaps without the
/// full text and graphics matrices.
struct Pen {
    size: f32,
    scale: f32,
    line_x: f32,
    x: f32,
    y: f32,
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
    /// Fonts, sizes and transformations saved by `q`, restored by `Q`.
    saved: Vec<(Option<&'d Dictionary>, f32, Matrix)>,
    lost: usize,
    operations_left: usize,
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
                "Td" | "TD" => self
                    .pen
                    .move_by(number(0).unwrap_or(0.0), number(1).unwrap_or(0.0)),
                "Tm" => {
                    let (a, e, f) = (number(0), number(4), number(5));
                    self.pen
                        .set(a.unwrap_or(1.0), e.unwrap_or(0.0), f.unwrap_or(0.0));
                }
                "T*" | "'" | "\"" => self.break_line(),
                "q" => self.saved.push((self.current, self.pen.size, self.pen.ctm)),
                "Q" => {
                    if let Some((font, size, ctm)) = self.saved.pop() {
                        self.current = font;
                        self.pen.size = size;
                        self.pen.ctm = ctm;
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
                        let (font, size, ctm) = (self.current, self.pen.size, self.pen.ctm);
                        let saved = std::mem::take(&mut self.saved);
                        self.pen.ctm = form.matrix.then(ctm);
                        let walked = self.walk(&form.operations, &inner, depth + 1);
                        (self.current, self.pen.size, self.pen.ctm) = (font, size, ctm);
                        self.saved = saved;
                        walked?;
                    }
                }
                "Tj" | "TJ" => {}
                _ => continue,
            }
            if matches!(op.operator.as_str(), "Tj" | "TJ" | "'" | "\"") {
                self.show(op);
            }
        }
        Ok(())
    }

    fn break_line(&mut self) {
        if self.lines.last().is_some_and(|l| !l.is_empty()) {
            self.lines.push(Vec::new());
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
            let raw_codes = simple && fonts.is_named_legacy(index);
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
                cmap: if raw_codes {
                    fonts.cmap(doc, dict)
                } else {
                    None
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
                    self.fonts.add_text(index, &raw, mapped.as_deref());
                    self.push(
                        Piece::Text {
                            font: index,
                            raw,
                            mapped,
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
