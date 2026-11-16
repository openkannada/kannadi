//! PDF files, read for their text and written as UTF-8 text. Text in legacy fonts
//! is converted; other text is kept. A legacy font is found by its name, or, when
//! the PDF hides the name, by its text: mostly Latin-1 glyph codes that convert
//! into clean Kannada syllables. A font named as legacy whose codes do not follow
//! the Nudi layout, as when a PDF maker renumbers a font subset, is reported and
//! left out.

use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use kannadi::legacy::{decode_windows_1252, nudi_to_unicode, windows_1252_byte, Options};
use lopdf::content::Content;
use lopdf::{Dictionary, Document, Encoding, Object};

use crate::convert::{convert, Direction, Issue, Kind, Settings};
use crate::report::{Report, Row};

/// The most a single page or font stream may decompress to.
const LIMIT: usize = 64 << 20;

pub fn convert_file(bytes: &[u8], settings: &Settings) -> Result<(Vec<u8>, Report)> {
    let Direction::ToUnicode(options) = settings.direction else {
        bail!("PDF files can only be converted to Unicode");
    };
    let doc = Document::load_mem(bytes).context("not a readable PDF")?;
    let mut fonts = Fonts::default();
    let mut pages = Vec::new();
    for (number, id) in doc.get_pages() {
        pages.push((number, read_page(&doc, id, &mut fonts, settings)?));
    }
    fonts.decide();

    let mut out = String::new();
    let mut report = Report::default();
    for (font, reason) in fonts.skipped() {
        report.rows.push(Row {
            location: format!("font {font}"),
            kind: Kind::NotConverted,
            detail: reason.to_string(),
            context: String::new(),
        });
    }
    for (i, (number, lines)) in pages.iter().enumerate() {
        if i > 0 {
            out.push('\u{0C}');
        }
        for (l, line) in lines.iter().enumerate() {
            let location = format!("page {number}, line {}", l + 1);
            out.push_str(&write_line(
                line,
                &fonts,
                &options,
                settings,
                &location,
                &mut report,
            ));
            out.push('\n');
        }
    }
    Ok((out.into_bytes(), report))
}

/// Text shown in 1 font, or a space the layout implies.
#[derive(Debug)]
enum Piece {
    Text { font: usize, text: String },
    Space,
}

type Line = Vec<Piece>;

const RENUMBERED: &str = "named as a Nudi or Baraha font, but its glyph codes do not follow \
     the layout; the PDF maker may have renumbered them";
const OTHER_LAYOUT: &str = "looks like legacy Kannada in a layout other than Nudi and Baraha";
const RENUMBERED_SUBSET: &str = "its glyph codes were renumbered and the PDF has no usable \
     character map, so its text cannot be read";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict {
    Legacy,
    Other,
    /// Legacy text that does not convert, for the reason given.
    Unreadable(&'static str),
}

#[derive(Debug)]
struct Font {
    name: String,
    named_legacy: bool,
    kannada_digits: bool,
    /// The text shown in this font across the document.
    text: String,
    verdict: Verdict,
}

#[derive(Debug, Default)]
struct Fonts {
    fonts: Vec<Font>,
    by_name: HashMap<String, usize>,
}

impl Fonts {
    fn index(&mut self, name: &str, settings: &Settings) -> usize {
        if let Some(&i) = self.by_name.get(name) {
            return i;
        }
        let plain = plain_name(name);
        self.fonts.push(Font {
            name: name.to_string(),
            named_legacy: settings.fonts.is_legacy(plain),
            kannada_digits: is_kannada_digit_font(plain),
            text: String::new(),
            verdict: Verdict::Other,
        });
        self.by_name.insert(name.to_string(), self.fonts.len() - 1);
        self.fonts.len() - 1
    }

    /// Decides which fonts hold legacy text, once all their text is known.
    fn decide(&mut self) {
        for font in &mut self.fonts {
            let clean = converts_cleanly(&font.text);
            font.verdict = match (font.named_legacy, clean, looks_legacy(&font.text)) {
                _ if looks_renumbered(&font.text) => Verdict::Unreadable(RENUMBERED_SUBSET),
                (true, Some(false), _) => Verdict::Unreadable(RENUMBERED),
                (true, _, _) => Verdict::Legacy,
                (false, Some(true), true) => Verdict::Legacy,
                (false, Some(false), true) => Verdict::Unreadable(OTHER_LAYOUT),
                _ => Verdict::Other,
            };
        }
    }

    fn skipped(&self) -> impl Iterator<Item = (&str, &str)> {
        self.fonts.iter().filter_map(|f| match f.verdict {
            Verdict::Unreadable(reason) => Some((f.name.as_str(), reason)),
            _ => None,
        })
    }
}

/// The font name without a subset tag (`ABCDEF+`) or a style suffix (`,Bold`).
fn plain_name(name: &str) -> &str {
    let name = match name.split_once('+') {
        Some((tag, rest)) if tag.len() == 6 && tag.bytes().all(|b| b.is_ascii_uppercase()) => rest,
        _ => name,
    };
    name.split(',').next().unwrap_or(name)
}

/// The `k` variants of the Nudi fonts, such as Nudi 04 k, draw Kannada digits at
/// the ASCII digit codes.
fn is_kannada_digit_font(name: &str) -> bool {
    let key: String = name
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase();
    key.strip_prefix("nudi").is_some_and(|rest| {
        rest.len() == 3 && rest[..2].bytes().all(|b| b.is_ascii_digit()) && rest.ends_with('k')
    })
}

/// Whether text in a font converts with problems in at most 1 syllable in 10, or
/// `None` if it has too few syllables to tell.
fn converts_cleanly(text: &str) -> Option<bool> {
    let converted = nudi_to_unicode(text, &Options::default());
    let syllables = kannadi::count_aksharas(&converted.text);
    if syllables < 10 {
        return None;
    }
    let problems = converted.unknown.len() + converted.malformed.len();
    Some(problems * 10 <= syllables)
}

/// Whether text reads as symbols, as when a font's codes were renumbered from `!`:
/// at least 1 in 7 of its letters and symbols are symbols that prose, numbers and
/// web addresses rarely use.
fn looks_renumbered(text: &str) -> bool {
    let letters = text.chars().filter(|c| c.is_alphabetic()).count();
    let symbols = text
        .chars()
        .filter(|c| "!\"#$%&*+<=>?@[\\]^_`{|}~".contains(*c))
        .count();
    letters >= 10 && symbols * 7 >= letters + symbols
}

/// Whether at least 3 in 10 visible characters are Windows-1252 characters above
/// ASCII, as in legacy Kannada and rarely in other text.
fn looks_legacy(text: &str) -> bool {
    let visible: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    let high = visible
        .iter()
        .filter(|&&c| !c.is_ascii() && windows_1252_byte(c).is_some())
        .count();
    visible.len() >= 10 && high * 10 >= visible.len() * 3
}

/// What the text state needs to know about a font on a page.
struct PageFont<'d> {
    index: usize,
    encoding: Option<Encoding<'d>>,
    /// A 1-byte font named as legacy: its string bytes are the glyph codes.
    raw_codes: bool,
    widths: Widths,
}

/// Glyph advances in thousandths of the font size.
enum Widths {
    Simple {
        first: u32,
        widths: Vec<f32>,
    },
    Cid {
        widths: HashMap<u32, f32>,
        default: f32,
    },
}

impl Widths {
    fn of(doc: &Document, font: &Dictionary) -> Self {
        let number = |o: &Object| {
            o.as_float()
                .ok()
                .or_else(|| o.as_i64().ok().map(|n| n as f32))
        };
        let resolve = |o: &Object| match o {
            Object::Reference(id) => doc.get_object(*id).ok().cloned(),
            other => Some(other.clone()),
        };
        let subtype = font.get(b"Subtype").and_then(Object::as_name).ok();
        if subtype != Some(b"Type0".as_slice()) {
            let first = font.get(b"FirstChar").ok().and_then(number).unwrap_or(0.0) as u32;
            let widths = font
                .get(b"Widths")
                .ok()
                .and_then(resolve)
                .and_then(|o| o.as_array().ok().cloned())
                .map(|a| a.iter().filter_map(number).collect())
                .unwrap_or_default();
            return Widths::Simple { first, widths };
        }
        let descendant = font
            .get(b"DescendantFonts")
            .ok()
            .and_then(resolve)
            .and_then(|o| o.as_array().ok().and_then(|a| a.first().cloned()))
            .and_then(|o| resolve(&o))
            .and_then(|o| o.as_dict().ok().cloned())
            .unwrap_or_default();
        let default = descendant
            .get(b"DW")
            .ok()
            .and_then(number)
            .unwrap_or(1000.0);
        let mut widths = HashMap::new();
        let list = descendant
            .get(b"W")
            .ok()
            .and_then(resolve)
            .and_then(|o| o.as_array().ok().cloned())
            .unwrap_or_default();
        let mut i = 0;
        while i + 1 < list.len() {
            let Some(start) = number(&list[i]) else { break };
            let start = start as u32;
            match resolve(&list[i + 1]) {
                Some(Object::Array(run)) => {
                    for (k, w) in run.iter().filter_map(number).enumerate() {
                        widths.insert(start + k as u32, w);
                    }
                    i += 2;
                }
                _ => {
                    let (Some(end), Some(w)) =
                        (number(&list[i + 1]), list.get(i + 2).and_then(number))
                    else {
                        break;
                    };
                    for c in start..=(end as u32).min(start + 65_535) {
                        widths.insert(c, w);
                    }
                    i += 3;
                }
            }
        }
        Widths::Cid { widths, default }
    }

    /// The advance of a shown string, in thousandths of the font size.
    fn advance(&self, bytes: &[u8]) -> f32 {
        match self {
            Widths::Simple { first, widths } => bytes
                .iter()
                .map(|&b| {
                    let i = u32::from(b).checked_sub(*first).map(|i| i as usize);
                    i.and_then(|i| widths.get(i)).copied().unwrap_or(500.0)
                })
                .sum(),
            Widths::Cid { widths, default } => bytes
                .chunks(2)
                .map(|c| {
                    let code = u32::from(c[0]) << 8 | u32::from(*c.get(1).unwrap_or(&0));
                    widths.get(&code).copied().unwrap_or(*default)
                })
                .sum(),
        }
    }
}

/// Where the next glyph goes, in the page's units, for finding line breaks and
/// word gaps without a full text matrix.
#[derive(Default)]
struct Pen {
    size: f32,
    scale: f32,
    line_x: f32,
    x: f32,
    y: f32,
    /// Where the last shown string ended, on this line.
    end: Option<f32>,
}

impl Pen {
    /// Whether a vertical move starts a new line: more than half the font size,
    /// since vowel signs and subscripts are often drawn a little off the baseline.
    fn is_new_line(&self, dy: f32) -> bool {
        dy.abs() > (0.5 * self.size * self.scale).max(0.5)
    }

    fn move_by(&mut self, tx: f32, ty: f32) -> bool {
        self.line_x += tx * self.scale;
        self.x = self.line_x;
        self.y += ty * self.scale;
        self.is_new_line(ty * self.scale)
    }

    fn set(&mut self, scale: f32, x: f32, y: f32) -> bool {
        let new_line = self.is_new_line(y - self.y);
        self.scale = if scale.abs() > 0.0 { scale.abs() } else { 1.0 };
        self.line_x = x;
        self.x = x;
        self.y = y;
        new_line
    }

    /// Whether a string starting here leaves a word gap after the last one.
    fn gap(&self) -> bool {
        self.end
            .is_some_and(|end| self.x - end > 0.2 * self.size * self.scale)
    }

    fn advance(&mut self, thousandths: f32) {
        self.x += thousandths / 1000.0 * self.size * self.scale;
        self.end = Some(self.x);
    }
}

fn read_page(
    doc: &Document,
    page: lopdf::ObjectId,
    fonts: &mut Fonts,
    settings: &Settings,
) -> Result<Vec<Line>> {
    let content = doc
        .get_page_content_with_limit(page, LIMIT)
        .context("reading a page")?;
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
        lines: vec![Vec::new()],
        pen: Pen {
            scale: 1.0,
            ..Pen::default()
        },
        current: None,
    };
    reader.walk(&content, &resources, 0)?;
    Ok(reader.lines)
}

/// Forms may draw other forms; deeper nesting is taken as a loop.
const MAX_FORM_DEPTH: usize = 8;

struct PageReader<'d, 'f> {
    doc: &'d Document,
    fonts: &'f mut Fonts,
    settings: &'f Settings,
    /// Fonts by their dictionary's address, since forms may reuse a name.
    page_fonts: HashMap<usize, PageFont<'d>>,
    lines: Vec<Line>,
    pen: Pen,
    current: Option<&'d Dictionary>,
}

impl<'d> PageReader<'d, '_> {
    /// Reads a content stream drawn with `resources`, and the forms it draws.
    fn walk(&mut self, content: &[u8], resources: &[&'d Dictionary], depth: usize) -> Result<()> {
        let content = Content::decode(content).context("reading a page's content")?;
        for op in &content.operations {
            let number = |i: usize| op.operands.get(i).and_then(|o| o.as_float().ok());
            let name = || op.operands.first().and_then(|o| o.as_name().ok());
            let mut break_line = false;
            match op.operator.as_str() {
                "Tf" => {
                    self.current = name().and_then(|n| self.resource(resources, b"Font", n));
                    self.pen.size = number(1).unwrap_or(self.pen.size);
                }
                "Td" | "TD" => {
                    break_line = self
                        .pen
                        .move_by(number(0).unwrap_or(0.0), number(1).unwrap_or(0.0))
                }
                "Tm" => {
                    let (a, e, f) = (number(0), number(4), number(5));
                    break_line = self
                        .pen
                        .set(a.unwrap_or(1.0), e.unwrap_or(0.0), f.unwrap_or(0.0));
                }
                "T*" | "'" | "\"" => break_line = true,
                "BT" => self.pen.end = None,
                "Do" if depth < MAX_FORM_DEPTH => {
                    if let Some(form) = name().and_then(|n| self.form(resources, n)) {
                        let (content, own) = form;
                        let mut inner = own.into_iter().collect::<Vec<_>>();
                        inner.extend_from_slice(resources);
                        self.walk(&content, &inner, depth + 1)?;
                    }
                }
                _ => {}
            }
            if break_line {
                new_line(&mut self.lines);
                self.pen.end = None;
            }
            if matches!(op.operator.as_str(), "Tj" | "TJ" | "'" | "\"") {
                self.show(op);
            }
        }
        Ok(())
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
            let entries = self.resolve(r.get(category).ok()?)?.as_dict().ok()?;
            self.resolve(entries.get(name).ok()?)?.as_dict().ok()
        })
    }

    /// A form XObject's decoded content and its own resources.
    fn form(
        &self,
        resources: &[&'d Dictionary],
        name: &[u8],
    ) -> Option<(Vec<u8>, Option<&'d Dictionary>)> {
        let stream = resources.iter().find_map(|r| {
            let entries = self.resolve(r.get(b"XObject").ok()?)?.as_dict().ok()?;
            self.resolve(entries.get(name).ok()?)?.as_stream().ok()
        })?;
        if stream.dict.get(b"Subtype").and_then(Object::as_name).ok() != Some(b"Form".as_slice()) {
            return None;
        }
        let content = stream.decompressed_content_with_limit(LIMIT).ok()?;
        let own = stream
            .dict
            .get(b"Resources")
            .ok()
            .and_then(|o| self.resolve(o))
            .and_then(|o| o.as_dict().ok());
        Some((content, own))
    }

    fn resolve(&self, object: &'d Object) -> Option<&'d Object> {
        match object {
            Object::Reference(id) => self.doc.get_object(*id).ok(),
            other => Some(other),
        }
    }

    fn show(&mut self, op: &lopdf::content::Operation) {
        let Some(dict) = self.current else { return };
        let key = std::ptr::from_ref(dict) as usize;
        let (doc, settings) = (self.doc, self.settings);
        let fonts = &mut *self.fonts;
        let font = self
            .page_fonts
            .entry(key)
            .or_insert_with(|| page_font(doc, dict, fonts, settings));
        for item in shown(op) {
            let line = self.lines.last_mut().expect("a line is always open");
            match item {
                Shown::Adjust(n) => {
                    if n < -200.0 {
                        line.push(Piece::Space);
                    }
                    self.pen.advance(-n);
                }
                Shown::Bytes(bytes) => {
                    if self.pen.gap() {
                        line.push(Piece::Space);
                        self.fonts.fonts[font.index].text.push(' ');
                    }
                    let text = if font.raw_codes {
                        decode_windows_1252(bytes)
                    } else {
                        font.encoding
                            .as_ref()
                            .and_then(|e| Document::decode_text(e, bytes).ok())
                            .unwrap_or_else(|| decode_windows_1252(bytes))
                    };
                    self.fonts.fonts[font.index].text.push_str(&text);
                    line.push(Piece::Text {
                        font: font.index,
                        text,
                    });
                    self.pen.advance(font.widths.advance(bytes));
                }
            }
        }
    }
}

fn page_font<'d>(
    doc: &'d Document,
    dict: &'d Dictionary,
    fonts: &mut Fonts,
    settings: &Settings,
) -> PageFont<'d> {
    let name = dict
        .get(b"BaseFont")
        .and_then(Object::as_name)
        .map(|n| String::from_utf8_lossy(n).to_string())
        .unwrap_or_default();
    let index = fonts.index(&name, settings);
    let simple = dict
        .get(b"Subtype")
        .and_then(Object::as_name)
        .is_ok_and(|s| s != b"Type0");
    PageFont {
        index,
        encoding: dict.get_font_encoding_with_limit(doc, LIMIT).ok(),
        raw_codes: simple && fonts.fonts[index].named_legacy,
        widths: Widths::of(doc, dict),
    }
}

enum Shown<'a> {
    Bytes(&'a [u8]),
    /// A `TJ` position adjustment, in thousandths of the font size; negative
    /// numbers move right.
    Adjust(f32),
}

fn shown(op: &lopdf::content::Operation) -> Vec<Shown<'_>> {
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

fn new_line(lines: &mut Vec<Line>) {
    if lines.last().is_some_and(|l| !l.is_empty()) {
        lines.push(Vec::new());
    }
}

/// 1 line of text: runs of legacy text are joined and converted together.
fn write_line(
    line: &[Piece],
    fonts: &Fonts,
    options: &Options,
    settings: &Settings,
    location: &str,
    report: &mut Report,
) -> String {
    let mut out = String::new();
    let mut legacy = String::new();
    let mut digits = false;
    let flush = |legacy: &mut String, digits: bool, out: &mut String, report: &mut Report| {
        if legacy.is_empty() {
            return;
        }
        let options = Options {
            kannada_digits: options.kannada_digits || digits,
            ..*options
        };
        let piece_settings = Settings {
            direction: Direction::ToUnicode(options),
            ..settings.clone()
        };
        let converted = convert(legacy, &piece_settings);
        add_rows(report, location, &converted.issues);
        out.push_str(&converted.text);
        legacy.clear();
    };
    for piece in line {
        match piece {
            Piece::Space if !legacy.is_empty() => legacy.push(' '),
            Piece::Space => out.push(' '),
            Piece::Text { font, text } => match fonts.fonts[*font].verdict {
                Verdict::Legacy => {
                    digits = fonts.fonts[*font].kannada_digits;
                    legacy.push_str(text);
                }
                Verdict::Unreadable(_) => flush(&mut legacy, digits, &mut out, report),
                Verdict::Other => {
                    flush(&mut legacy, digits, &mut out, report);
                    out.push_str(text);
                }
            },
        }
    }
    flush(&mut legacy, digits, &mut out, report);
    out.trim_end().to_string()
}

fn add_rows(report: &mut Report, location: &str, issues: &[Issue]) {
    report
        .rows
        .extend(issues.iter().map(|i| Row::new(location.to_string(), i)));
}

#[cfg(test)]
mod tests {
    use super::*;
    use kannadi::legacy::Options;
    use lopdf::{dictionary, Stream};

    /// A 1-page PDF with the given fonts and content stream.
    fn pdf(fonts: Vec<(&str, Dictionary)>, content: &[u8]) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages = doc.new_object_id();
        let mut font_refs = Dictionary::new();
        for (name, font) in fonts {
            let id = doc.add_object(font);
            font_refs.set(name, Object::Reference(id));
        }
        let contents = doc.add_object(Stream::new(dictionary! {}, content.to_vec()));
        let page = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages,
            "Contents" => contents,
            "Resources" => dictionary! { "Font" => font_refs },
        });
        doc.objects.insert(
            pages,
            Object::Dictionary(
                dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 },
            ),
        );
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", catalog);
        let mut out = Vec::new();
        doc.save_to(&mut out).unwrap();
        out
    }

    fn simple_font(name: &str) -> Dictionary {
        dictionary! {
            "Type" => "Font",
            "Subtype" => "TrueType",
            "BaseFont" => name,
            "Encoding" => "WinAnsiEncoding",
            "FirstChar" => 32,
            "Widths" => vec![Object::Integer(500); 224],
        }
    }

    fn run(bytes: &[u8]) -> (String, Report) {
        let settings = Settings::new(Direction::ToUnicode(Options::default()));
        let (out, report) = convert_file(bytes, &settings).unwrap();
        (String::from_utf8(out).unwrap(), report)
    }

    const KANNADA: &[u8] = b"P\xC0\xA3\xC0\xDFq\xC0";

    #[test]
    fn named_legacy_fonts_convert_and_others_stay() {
        let mut content = b"BT /F1 12 Tf 72 700 Td (".to_vec();
        content.extend_from_slice(KANNADA);
        content.extend_from_slice(b") Tj /F2 12 Tf 60 0 Td (Office) Tj ET");
        let fonts = vec![
            ("F1", simple_font("ABCDEF+BRHKannada")),
            ("F2", simple_font("Arial")),
        ];
        let (text, report) = run(&pdf(fonts, &content));
        assert_eq!(text, "ಕನ್ನಡ Office\n");
        assert!(report.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn named_legacy_fonts_without_an_encoding_give_raw_codes() {
        let mut font = simple_font("BRHKannada");
        font.remove(b"Encoding");
        let mut content = b"BT /F1 12 Tf 72 700 Td (".to_vec();
        content.extend_from_slice(KANNADA);
        content.extend_from_slice(b") Tj ET");
        let (text, _) = run(&pdf(vec![("F1", font)], &content));
        assert_eq!(text, "ಕನ್ನಡ\n");
    }

    #[test]
    fn fragments_join_and_wide_gaps_are_spaces() {
        // Each glyph is 6 units wide at 12 points: "P\xC0" ends at 12.
        let content =
            b"BT /F1 12 Tf 72 700 Td (P\xC0) Tj 12 0 Td (\xA3\xC0\xDF) Tj 18 0 Td (q\xC0) Tj \
                        30 0 Td (P\xC0) Tj 0 -14 Td (q\xC0) Tj ET";
        let (text, _) = run(&pdf(vec![("F1", simple_font("BRHKannada"))], content));
        assert_eq!(text, "ಕನ್ನಡ ಕ\nಡ\n");
    }

    #[test]
    fn a_font_with_a_hidden_name_is_found_by_its_text() {
        let (doc, font) = cid_font(None);
        let shown = "<0001000200030002000400010002>".repeat(6);
        let content = format!("BT /F1 12 Tf 72 700 Td {shown} Tj ET");
        let (text, report) = run(&pdf_with_objects(doc, vec![font], content.as_bytes()));
        assert!(text.starts_with("ಕನ ಕಕನ ಕ"), "{text:?}");
        assert!(report.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn cid_widths_place_word_gaps() {
        // With W, the 8 glyphs of a word take 24 units at 12 points; with DW alone, 48.
        let (doc, font) = cid_font(Some(vec![1.into(), vec![Object::Integer(250); 4].into()]));
        let word = "<00010002000300020001000200030002>";
        let shown = format!("{word} Tj 30 0 Td ").repeat(3);
        let content = format!("BT /F1 12 Tf 72 700 Td {shown} ET");
        let (text, _) = run(&pdf_with_objects(doc, vec![font], content.as_bytes()));
        assert!(text.starts_with("ಕನಕನ ಕನಕನ"), "{text:?}");
    }

    /// A CID font with a hidden name whose ToUnicode map gives Nudi codes for
    /// CIDs 1 to 4: P, À, £ and a space.
    fn cid_font(widths: Option<Vec<Object>>) -> (Document, Dictionary) {
        let cmap = b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
4 beginbfchar\n<0001> <0050>\n<0002> <00C0>\n<0003> <00A3>\n<0004> <0020>\nendbfchar\n\
endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n";
        let mut descendant = dictionary! {
            "Type" => "Font", "Subtype" => "CIDFontType2", "BaseFont" => "CIDFont+F1", "DW" => 500,
        };
        if let Some(w) = widths {
            descendant.set("W", w);
        }
        let mut doc = Document::with_version("1.5");
        let d = doc.add_object(descendant);
        let t = doc.add_object(Stream::new(dictionary! {}, cmap.to_vec()));
        let font = dictionary! {
            "Type" => "Font", "Subtype" => "Type0", "BaseFont" => "CIDFont+F1",
            "Encoding" => "Identity-H", "DescendantFonts" => vec![d.into()], "ToUnicode" => t,
        };
        (doc, font)
    }

    #[test]
    fn text_in_forms_is_read() {
        let mut doc = Document::with_version("1.5");
        let font = doc.add_object(simple_font("BRHKannada"));
        let mut form_text = b"BT /K 12 Tf 72 700 Td (".to_vec();
        form_text.extend_from_slice(KANNADA);
        form_text.extend_from_slice(b") Tj ET");
        let form = doc.add_object(Stream::new(
            dictionary! {
                "Type" => "XObject", "Subtype" => "Form",
                "Resources" => dictionary! { "Font" => dictionary! { "K" => font } },
            },
            form_text,
        ));
        let pages = doc.new_object_id();
        let contents = doc.add_object(Stream::new(dictionary! {}, b"/X1 Do /X1 Do".to_vec()));
        let page = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages, "Contents" => contents,
            "Resources" => dictionary! { "XObject" => dictionary! { "X1" => form } },
        });
        doc.objects.insert(
            pages,
            Object::Dictionary(
                dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 },
            ),
        );
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", catalog);
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        let (text, _) = run(&bytes);
        assert_eq!(text, "ಕನ್ನಡ\nಕನ್ನಡ\n");
    }

    #[test]
    fn small_vertical_moves_and_tj_gaps() {
        let content =
            b"BT /F1 12 Tf 72 700 Td (P) Tj 6 -2 Td (\xC0) Tj [(P\xC0) -300 (P\xC0)] TJ ET";
        let (text, report) = run(&pdf(vec![("F1", simple_font("BRHKannada"))], content));
        assert_eq!(text, "ಕಕ ಕ\n");
        assert!(report.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn ascii_text_in_other_fonts_is_never_converted() {
        let content = b"BT /F1 12 Tf 72 700 Td (Qa Qa Qa Qa Qa Qa Qa) Tj ET";
        let (text, _) = run(&pdf(vec![("F1", simple_font("Arial"))], content));
        assert_eq!(text, "Qa Qa Qa Qa Qa Qa Qa\n");
    }

    #[test]
    fn a_named_font_whose_text_does_not_convert_is_reported() {
        let content =
            b"BT /F1 12 Tf 72 700 Td (The quick brown fox jumps over the lazy dog again) Tj ET";
        let (_, report) = run(&pdf(vec![("F1", simple_font("BRH Kannada"))], content));
        assert_eq!(report.rows[0].detail, RENUMBERED);
    }

    /// Like `pdf`, for fonts whose parts were added to `doc` already.
    fn pdf_with_objects(mut doc: Document, fonts: Vec<Dictionary>, content: &[u8]) -> Vec<u8> {
        let pages = doc.new_object_id();
        let mut font_refs = Dictionary::new();
        for (i, font) in fonts.into_iter().enumerate() {
            let id = doc.add_object(font);
            font_refs.set(format!("F{}", i + 1), Object::Reference(id));
        }
        let contents = doc.add_object(Stream::new(dictionary! {}, content.to_vec()));
        let page = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages, "Contents" => contents,
            "Resources" => dictionary! { "Font" => font_refs },
        });
        doc.objects.insert(
            pages,
            Object::Dictionary(
                dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 },
            ),
        );
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", catalog);
        let mut out = Vec::new();
        doc.save_to(&mut out).unwrap();
        out
    }

    #[test]
    fn unreadable_fonts_are_reported_and_left_out() {
        let renumbered =
            b"BT /F1 12 Tf 72 700 Td (!\"#$%&'()*+,-./!\"#$%&'()*+,-./ABCDEFGHIJ) Tj ET";
        let (text, report) = run(&pdf(vec![("F1", simple_font("Nudi01k"))], renumbered));
        assert_eq!(text, "\n");
        assert_eq!(report.rows[0].kind, Kind::NotConverted);
        assert_eq!(report.rows[0].location, "font Nudi01k");
        let other_layout = b"BT /F1 12 Tf 72 700 Td (p\xEA\xB3\xA9\xEEd\xD2 \xC0\xE2\xB0y\xE2\xB0K {\xEA\xA9\xC6d\xD6p\xD6\xB9 p\xEA\xB3\xA9\xEEd\xD2) Tj ET";
        let (_, report) = run(&pdf(
            vec![("F1", simple_font("AkrutiKndPadma"))],
            other_layout,
        ));
        assert_eq!(report.rows[0].detail, OTHER_LAYOUT);
    }

    #[test]
    fn renumbered_text_is_told_from_addresses_and_dates() {
        assert!(looks_renumbered(
            "!\"! #$$ %&%&%& )*+,-*!*.*! /-01&-01& KLMC'KLMC' WXYZQUQV[R\\Q]"
        ));
        assert!(!looks_renumbered(
            "www.advgen.kar.nic.in 560 001. 16000-400-17200 85/2015-16, 09.07.2015"
        ));
        assert!(!looks_renumbered(
            "Sultan & Another Vs. U.P. Public Service Commission (C) No. 2/2024"
        ));
    }

    #[test]
    fn k_fonts_give_kannada_digits() {
        let content = b"BT /F1 12 Tf 72 700 Td (P\xC0 2025) Tj ET";
        let (text, _) = run(&pdf(
            vec![("F1", simple_font("ABCDEF+Nudi 04 k,Bold"))],
            content,
        ));
        assert_eq!(text, "ಕ ೨೦೨೫\n");
        assert!(is_kannada_digit_font("Nudi 04 k"));
        assert!(!is_kannada_digit_font("Nudi 01 e"));
        assert_eq!(plain_name("ABCDEF+Nudi 04 k,Bold"), "Nudi 04 k");
    }

    #[test]
    fn only_conversion_to_unicode_is_possible() {
        let settings = Settings::new(Direction::ToNudi);
        assert!(convert_file(&pdf(vec![], b""), &settings).is_err());
        assert!(run_err(b"not a pdf"));
    }

    fn run_err(bytes: &[u8]) -> bool {
        let settings = Settings::new(Direction::ToUnicode(Options::default()));
        convert_file(bytes, &settings).is_err()
    }
}
