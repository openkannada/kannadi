//! Finds the paragraphs, runs and text of 1 DOCX part by position, so the part
//! can be edited in place.

use std::ops::Range;

use anyhow::Result;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::convert::Fonts;

use super::styles::legacy_font;
use super::xml::attr;

#[derive(Debug, Default)]
pub struct Paragraph {
    /// 1-based, in document order, not counting fallback copies of text boxes.
    pub number: usize,
    /// False for the fallback copy of a text box, whose problems the main copy
    /// already reports.
    pub reported: bool,
    pub style: Option<String>,
    pub runs: Vec<Run>,
}

#[derive(Debug, Default)]
pub struct Run {
    /// Just after `<w:r ...>`.
    pub content_start: usize,
    pub properties: Option<Properties>,
    pub pieces: Vec<Piece>,
    /// How many elements enclose the `w:r`.
    depth: usize,
}

/// An element in a run's properties, with its attributes as written.
#[derive(Debug, Clone)]
pub struct Element {
    pub span: Range<usize>,
    pub attributes: String,
}

/// A run's `w:rPr`.
#[derive(Debug, Default)]
pub struct Properties {
    pub span: Range<usize>,
    /// Written as `<w:rPr/>`.
    pub empty: bool,
    /// Just after `<w:rPr>`.
    pub content_start: usize,
    pub style: Option<String>,
    pub style_element: Option<Element>,
    pub fonts: Option<Element>,
    /// What the run's own `w:rFonts` says, if it names a Latin font.
    pub legacy: Option<bool>,
    pub bold: Option<Element>,
    pub bold_cs: Option<Element>,
    pub italic: Option<Element>,
    pub italic_cs: Option<Element>,
    pub size: Option<Element>,
    pub size_cs: Option<Element>,
}

#[derive(Debug)]
pub enum Piece {
    Text {
        /// `w:t`, or `w:delText` in a tracked deletion.
        element: &'static str,
        tag: Range<usize>,
        content: Range<usize>,
    },
    /// A tab or line break, which no syllable crosses.
    Break,
}

/// Elements whose children can be runs.
const RUN_PARENTS: [&str; 12] = [
    "w:p",
    "w:hyperlink",
    "w:ins",
    "w:del",
    "w:moveTo",
    "w:moveFrom",
    "w:smartTag",
    "w:customXml",
    "w:sdtContent",
    "w:fldSimple",
    "w:bdo",
    "w:dir",
];

/// The paragraphs of a part in document order.
pub fn scan(source: &str, fonts: &Fonts) -> Result<Vec<Paragraph>> {
    let mut scanner = Scanner {
        fonts,
        names: Vec::new(),
        paragraphs: Vec::new(),
        runs: Vec::new(),
        done: Vec::new(),
        count: 0,
        fallback: 0,
        text: None,
    };
    let mut reader = Reader::from_str(source);
    loop {
        let start = position(&reader);
        let event = reader.read_event()?;
        let end = position(&reader);
        match event {
            Event::Eof => break,
            Event::Start(e) => {
                scanner.open(&e, start..end, false);
                scanner.names.push(e.name().as_ref().to_string());
            }
            Event::Empty(e) => {
                scanner.open(&e, start..end, true);
                scanner.close(e.name().as_ref(), end, end);
            }
            Event::End(e) => {
                scanner.names.pop();
                scanner.close(e.name().as_ref(), start, end);
            }
            _ => {}
        }
    }
    scanner.done.sort_by_key(|p| p.number);
    Ok(scanner.done)
}

struct Scanner<'f> {
    fonts: &'f Fonts,
    names: Vec<String>,
    paragraphs: Vec<Paragraph>,
    runs: Vec<Run>,
    done: Vec<Paragraph>,
    count: usize,
    /// How many `mc:Fallback` elements are open.
    fallback: usize,
    /// The open `w:t`: its name, its start tag and where its content starts.
    text: Option<(&'static str, Range<usize>, usize)>,
}

impl Scanner<'_> {
    fn parent(&self) -> Option<&str> {
        self.names.last().map(String::as_str)
    }

    fn grandparent(&self) -> Option<&str> {
        let n = self.names.len();
        n.checked_sub(2).map(|i| self.names[i].as_str())
    }

    /// The open run, if the element being read is its child.
    fn run_child(&mut self) -> Option<&mut Run> {
        let depth = self.names.len();
        self.runs.last_mut().filter(|r| r.depth + 1 == depth)
    }

    /// The open run's properties, if the element being read is in them.
    fn property(&mut self) -> Option<&mut Properties> {
        let depth = self.names.len();
        let in_properties = self.parent() == Some("w:rPr");
        self.runs
            .last_mut()
            .filter(|r| in_properties && r.depth + 2 == depth)
            .and_then(|r| r.properties.as_mut())
    }

    fn open(&mut self, e: &BytesStart, span: Range<usize>, empty: bool) {
        let element = || Element {
            span: span.clone(),
            attributes: e.attributes_raw().to_string(),
        };
        match e.name().as_ref() {
            "mc:Fallback" if !empty => self.fallback += 1,
            "w:p" => {
                let reported = self.fallback == 0;
                if reported {
                    self.count += 1;
                }
                self.paragraphs.push(Paragraph {
                    number: self.count,
                    reported,
                    ..Paragraph::default()
                });
            }
            "w:pStyle" if self.parent() == Some("w:pPr") && self.grandparent() == Some("w:p") => {
                if let Some(p) = self.paragraphs.last_mut() {
                    p.style = attr(e, "w:val");
                }
            }
            "w:r" if self.parent().is_some_and(|p| RUN_PARENTS.contains(&p)) => {
                let depth = self.names.len();
                self.runs.push(Run {
                    content_start: span.end,
                    depth,
                    ..Run::default()
                });
            }
            "w:rPr" => {
                if let Some(run) = self.run_child() {
                    run.properties = Some(Properties {
                        span: span.clone(),
                        empty,
                        content_start: span.end,
                        ..Properties::default()
                    });
                }
            }
            name @ ("w:t" | "w:delText") if !empty => {
                let element = if name == "w:t" { "w:t" } else { "w:delText" };
                if self.run_child().is_some() {
                    self.text = Some((element, span.clone(), span.end));
                }
            }
            "w:tab" | "w:br" | "w:cr" => {
                if let Some(run) = self.run_child() {
                    run.pieces.push(Piece::Break);
                }
            }
            name => {
                let legacy = (name == "w:rFonts")
                    .then(|| legacy_font(e, self.fonts))
                    .flatten();
                let style = (name == "w:rStyle").then(|| attr(e, "w:val")).flatten();
                let Some(p) = self.property() else { return };
                let slot = match name {
                    "w:rStyle" => {
                        p.style = style;
                        &mut p.style_element
                    }
                    "w:rFonts" => {
                        p.legacy = legacy;
                        &mut p.fonts
                    }
                    "w:b" => &mut p.bold,
                    "w:bCs" => &mut p.bold_cs,
                    "w:i" => &mut p.italic,
                    "w:iCs" => &mut p.italic_cs,
                    "w:sz" => &mut p.size,
                    "w:szCs" => &mut p.size_cs,
                    _ => return,
                };
                *slot = Some(element());
            }
        }
    }

    /// Ends the element named `name`, whose end tag runs from `start` to `end`.
    fn close(&mut self, name: &str, start: usize, end: usize) {
        match name {
            "mc:Fallback" if start != end => self.fallback = self.fallback.saturating_sub(1),
            "w:t" | "w:delText" => {
                if let (Some((element, tag, content_start)), Some(run)) =
                    (self.text.take(), self.runs.last_mut())
                {
                    run.pieces.push(Piece::Text {
                        element,
                        tag,
                        content: content_start..start,
                    });
                }
            }
            "w:rPr" => {
                if let Some(p) = self.run_child().and_then(|r| r.properties.as_mut()) {
                    p.span.end = end;
                }
            }
            "w:r" => {
                let depth = self.names.len();
                if self.runs.last().is_some_and(|r| r.depth == depth) {
                    if let (Some(run), Some(p)) = (self.runs.pop(), self.paragraphs.last_mut()) {
                        p.runs.push(run);
                    }
                }
            }
            "w:p" => self.done.extend(self.paragraphs.pop()),
            _ => {
                if let Some(p) = self.property() {
                    let slot = match name {
                        "w:rStyle" => &mut p.style_element,
                        "w:rFonts" => &mut p.fonts,
                        "w:b" => &mut p.bold,
                        "w:bCs" => &mut p.bold_cs,
                        "w:i" => &mut p.italic,
                        "w:iCs" => &mut p.italic_cs,
                        "w:sz" => &mut p.size,
                        "w:szCs" => &mut p.size_cs,
                        _ => return,
                    };
                    if let Some(element) = slot {
                        element.span.end = end;
                    }
                }
            }
        }
    }
}

fn position(reader: &Reader<&[u8]>) -> usize {
    usize::try_from(reader.buffer_position()).unwrap_or(usize::MAX)
}
