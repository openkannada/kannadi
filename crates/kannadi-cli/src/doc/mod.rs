//! Word 97 to 2003 documents (`.doc`), read for their text and each run's font,
//! following [MS-DOC]. The text comes from the piece table; a run's font comes
//! from its paragraph style, its character style, its own properties and its
//! piece, in that order, each over the last.

#[cfg(test)]
pub mod build;
mod bytes;
mod fib;
mod sprm;
mod tables;

use std::io::{Cursor, Read};

use anyhow::{bail, Context, Result};

use crate::flow::{Document, Story};
use fib::{Fib, HEADERS, STORIES};
use sprm::CharProps;
use tables::{ParagraphProps, Piece, Spans, Styles};

/// The most text a document may hold, in characters, and the most a stream may hold.
const MAX_CHARS: u64 = 1 << 26;
const MAX_STREAM: u64 = 1 << 28;

/// Reads the stories of a `.doc` file. Fails on files that are not Word 97 or
/// later, or are password-protected.
pub fn read(bytes: &[u8]) -> Result<Document> {
    let mut file = cfb::CompoundFile::open(Cursor::new(bytes))
        .context("not a Word document (no OLE compound file)")?;
    let word = stream(&mut file, "/WordDocument")?;
    let fib = Fib::parse(&word)?;
    let table = stream(&mut file, if fib.table_1 { "/1Table" } else { "/0Table" })?;
    let part = |(fc, lcb): (u32, u32)| -> &[u8] {
        bytes::slice(&table, fc as usize, lcb as usize).unwrap_or_default()
    };
    let pieces = tables::pieces(part(fib.clx)).context("reading the piece table")?;
    let reader = Reader {
        word: &word,
        pieces: &pieces,
        chars: tables::char_spans(part(fib.chpx), &word),
        paragraphs: tables::paragraph_spans(part(fib.papx), &word),
        styles: Styles::parse(part(fib.stsh)),
    };
    let total: u64 = fib.ccp.iter().map(|&n| u64::from(n)).sum();
    if total > MAX_CHARS {
        bail!("the document says it holds {total} characters, more than this tool reads");
    }
    let mut doc = Document {
        fonts: tables::fonts(part(fib.fonts)),
        stories: Vec::new(),
    };
    let mut cp = 0u32;
    for (i, ((name, _), ccp)) in STORIES.into_iter().zip(fib.ccp).enumerate() {
        let end = cp.saturating_add(ccp);
        let story = if i == HEADERS {
            reader.headers(name, cp, end, part(fib.hdd))
        } else {
            reader.story(name, cp, end)
        };
        if !story.is_empty() {
            doc.stories.push(story);
        }
        cp = end;
    }
    Ok(doc)
}

fn stream(file: &mut cfb::CompoundFile<Cursor<&[u8]>>, name: &str) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    file.open_stream(name)
        .with_context(|| format!("no {} stream", name.trim_start_matches('/')))?
        .take(MAX_STREAM)
        .read_to_end(&mut out)
        .with_context(|| format!("reading the {} stream", name.trim_start_matches('/')))?;
    Ok(out)
}

struct Reader<'a> {
    word: &'a [u8],
    pieces: &'a [Piece],
    chars: Spans<CharProps>,
    paragraphs: Spans<ParagraphProps>,
    styles: Styles,
}

/// What a paragraph holds until its mark gives its style.
enum Item {
    Char(char, CharProps),
    Break,
}

/// Where a field's characters are: its instructions are hidden, its result shown.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Field {
    Code,
    Result,
}

impl Reader<'_> {
    /// The story at character positions `start..end`.
    fn story(&self, name: &'static str, start: u32, end: u32) -> Story {
        let mut story = Story::new(name);
        let mut pending: Vec<Item> = Vec::new();
        let mut fields: Vec<Field> = Vec::new();
        for (c, fc, props) in self.chars(start, end) {
            match c {
                '\u{13}' => fields.push(Field::Code),
                '\u{14}' => {
                    if let Some(top) = fields.last_mut() {
                        *top = Field::Result;
                    }
                }
                '\u{15}' => {
                    fields.pop();
                }
                // Paragraph mark, and the mark ending a table cell or row. A field's
                // instructions never hold one, so an unclosed field ends here.
                '\r' | '\u{07}' => {
                    fields.retain(|&f| f == Field::Result);
                    let paragraph = self.paragraphs.at(fc);
                    self.close(&mut story, &mut pending, paragraph.style);
                    if !(c == '\u{07}' && paragraph.row_end) {
                        story.end_paragraph();
                    }
                }
                _ if fields.contains(&Field::Code) => {}
                // Line, page and column breaks.
                '\u{0B}' | '\u{0C}' | '\u{0E}' => pending.push(Item::Break),
                // A non-breaking hyphen.
                '\u{1E}' => pending.push(Item::Char('-', props)),
                '\t' => pending.push(Item::Char(c, props)),
                // Pictures, note references, optional hyphens and other marks.
                _ if c < ' ' => {}
                _ => pending.push(Item::Char(c, props)),
            }
        }
        self.close(&mut story, &mut pending, 0);
        story.finish();
        story
    }

    /// The headers and footers at `start..end`, split where the PlcfHdd `plc` says
    /// ([MS-DOC] 2.9.193). Each with text is written once, however many sections or
    /// pages repeat it.
    fn headers(&self, name: &'static str, start: u32, end: u32, plc: &[u8]) -> Story {
        let bounds: Vec<u32> = plc
            .as_chunks::<4>()
            .0
            .iter()
            .map(|&b| u32::from_le_bytes(b))
            .collect();
        if bounds.len() < 2 {
            return self.story(name, start, end);
        }
        let mut out = Story::new(name);
        out.paragraphs.clear();
        let mut seen = Vec::new();
        let mut done = start;
        for pair in bounds.windows(2) {
            let from = start.saturating_add(pair[0]).max(done);
            let to = start.saturating_add(pair[1]).min(end);
            if from >= to {
                continue;
            }
            done = to;
            let part = self.story(name, from, to);
            if part.is_empty() || seen.contains(&part.paragraphs) {
                continue;
            }
            out.paragraphs.extend(part.paragraphs.iter().cloned());
            seen.push(part.paragraphs);
        }
        if out.paragraphs.is_empty() {
            out.paragraphs.push(Vec::new());
        }
        out
    }

    /// Adds the pending items to `story`, their fonts read over paragraph style `istd`.
    fn close(&self, story: &mut Story, pending: &mut Vec<Item>, istd: u16) {
        let paragraph = self.styles.defaults.and(self.styles.props(istd));
        for item in pending.drain(..) {
            match item {
                Item::Break => story.end_paragraph(),
                Item::Char(c, direct) => {
                    let character = direct.style.map(|s| self.styles.props(s));
                    let props = paragraph.and(character.unwrap_or_default()).and(direct);
                    story.push(props.font_for(c).map(usize::from), c);
                }
            }
        }
    }

    /// Each character at positions `start..end` with its stream offset and its own
    /// properties: those of its run, then of its piece.
    fn chars(&self, start: u32, end: u32) -> impl Iterator<Item = (char, u32, CharProps)> + '_ {
        let from = self.pieces.partition_point(|p| p.cp_end <= start);
        self.pieces[from..]
            .iter()
            .take_while(move |p| p.cp_start < end)
            .flat_map(move |piece| {
                let first = start.max(piece.cp_start) - piece.cp_start;
                let last = end.min(piece.cp_end) - piece.cp_start;
                self.piece_chars(piece, first, last)
            })
    }

    fn piece_chars(
        &self,
        piece: &Piece,
        first: u32,
        last: u32,
    ) -> impl Iterator<Item = (char, u32, CharProps)> + '_ {
        let width = if piece.compressed { 1 } else { 2 };
        let at = |i: u32| piece.offset.checked_add(i.checked_mul(width)?);
        let units: Vec<(u32, u16)> = (first..last)
            .map_while(|i| {
                let fc = at(i)?;
                let unit = if piece.compressed {
                    u16::from(*self.word.get(fc as usize)?)
                } else {
                    bytes::u16_at(self.word, fc as usize)?
                };
                Some((fc, unit))
            })
            .collect();
        let compressed = piece.compressed;
        let props = piece.props;
        decode(units, compressed)
            .into_iter()
            .map(move |(c, fc)| (c, fc, self.chars.at(fc).and(props)))
    }
}

/// Characters from 8-bit Windows-1252 units or UTF-16 units, each with the
/// offset of its first unit. An unpaired surrogate becomes U+FFFD.
fn decode(units: Vec<(u32, u16)>, compressed: bool) -> Vec<(char, u32)> {
    if compressed {
        return units
            .into_iter()
            .map(|(fc, u)| {
                let byte = [u as u8];
                let c = kannadi::legacy::decode_windows_1252(&byte)
                    .chars()
                    .next()
                    .unwrap_or('\u{FFFD}');
                (c, fc)
            })
            .collect();
    }
    let offsets: Vec<u32> = units.iter().map(|&(fc, _)| fc).collect();
    let mut out = Vec::with_capacity(units.len());
    let mut i = 0;
    for c in char::decode_utf16(units.iter().map(|&(_, u)| u)) {
        let c = c.unwrap_or(char::REPLACEMENT_CHARACTER);
        out.push((c, offsets[i]));
        i += c.len_utf16().min(units.len() - i);
    }
    out
}

#[cfg(test)]
mod tests;
