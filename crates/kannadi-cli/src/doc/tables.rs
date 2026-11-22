//! The tables a Word 97 document keeps in its table stream: the piece table, the
//! character and paragraph property pages, the stylesheet and the font table.

use anyhow::{bail, Result};

use super::bytes::{slice, u16_at, u32_at, u8_at};
use super::sprm::{ends_row, CharProps};
use crate::flow::Font;

/// A run of characters stored together in the WordDocument stream ([MS-DOC] 2.9.177).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    pub cp_start: u32,
    pub cp_end: u32,
    /// Byte offset of the first character in the WordDocument stream.
    pub offset: u32,
    /// 1 byte per character in Windows-1252, else 2 in UTF-16.
    pub compressed: bool,
    /// Properties the piece itself applies to its characters.
    pub props: CharProps,
}

/// The piece table from the Clx ([MS-DOC] 2.9.38). Prc entries give a piece's
/// own properties through a Prm1.
pub fn pieces(clx: &[u8]) -> Result<Vec<Piece>> {
    let mut grpprls: Vec<&[u8]> = Vec::new();
    let mut at = 0;
    let plc = loop {
        match u8_at(clx, at) {
            Some(0x01) => {
                let cb = usize::from(u16_at(clx, at + 1).unwrap_or(0));
                let Some(grpprl) = slice(clx, at + 3, cb) else {
                    bail!("the piece table is cut short");
                };
                grpprls.push(grpprl);
                at += 3 + cb;
            }
            Some(0x02) => {
                let lcb = u32_at(clx, at + 1).unwrap_or(0) as usize;
                match slice(clx, at + 5, lcb) {
                    Some(plc) => break plc,
                    None => bail!("the piece table is cut short"),
                }
            }
            _ => bail!("no piece table"),
        }
    };
    if plc.len() < 4 || (plc.len() - 4) % 12 != 0 {
        bail!("the piece table has a bad length");
    }
    let n = (plc.len() - 4) / 12;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let cp = |j: usize| u32_at(plc, j * 4).unwrap_or(0);
        let pcd = 4 * (n + 1) + i * 8;
        let fc = u32_at(plc, pcd + 2).unwrap_or(0);
        let prm = u16_at(plc, pcd + 6).unwrap_or(0);
        let compressed = fc & 0x4000_0000 != 0;
        let fc = fc & 0x3FFF_FFFF;
        let props = if prm & 1 == 1 {
            grpprls
                .get(usize::from(prm >> 1))
                .map(|g| CharProps::parse(g))
                .unwrap_or_default()
        } else {
            CharProps::default()
        };
        let (cp_start, cp_end) = (cp(i), cp(i + 1));
        if cp_end < cp_start || out.last().is_some_and(|p: &Piece| p.cp_end > cp_start) {
            bail!("the piece table is out of order");
        }
        out.push(Piece {
            cp_start,
            cp_end,
            offset: if compressed { fc / 2 } else { fc },
            compressed,
            props,
        });
    }
    Ok(out)
}

/// Spans of the WordDocument stream, from a PlcBte and its 512-byte pages
/// ([MS-DOC] 2.8.6, 2.9.43 and 2.9.179), each with a value, sorted by start.
pub struct Spans<T> {
    spans: Vec<(u32, u32, T)>,
}

impl<T: Copy + Default> Spans<T> {
    /// The value of the span holding byte `fc`, or the default outside every span.
    pub fn at(&self, fc: u32) -> T {
        let i = self.spans.partition_point(|s| s.0 <= fc);
        match i.checked_sub(1).map(|i| &self.spans[i]) {
            Some(&(_, end, value)) if fc < end => value,
            _ => T::default(),
        }
    }
}

/// The page numbers a PlcBteChpx or PlcBtePapx lists.
fn pages(plc: &[u8]) -> Vec<usize> {
    if plc.len() < 4 {
        return Vec::new();
    }
    let n = (plc.len() - 4) / 8;
    (0..n)
        .filter_map(|i| u32_at(plc, 4 * (n + 1) + i * 4))
        .map(|pn| (pn & 0x003F_FFFF) as usize)
        .collect()
}

/// Character properties by stream offset, from the ChpxFkp pages.
pub fn char_spans(plc: &[u8], word: &[u8]) -> Spans<CharProps> {
    let mut spans = Vec::new();
    for pn in pages(plc) {
        let Some(page) = pn.checked_mul(512).and_then(|at| slice(word, at, 512)) else {
            continue;
        };
        let runs = usize::from(page[511]);
        for i in 0..runs {
            let (Some(start), Some(end)) = (u32_at(page, i * 4), u32_at(page, i * 4 + 4)) else {
                break;
            };
            let at = usize::from(u8_at(page, 4 * (runs + 1) + i).unwrap_or(0)) * 2;
            let props = match at {
                0 => CharProps::default(),
                _ => {
                    let cb = usize::from(u8_at(page, at).unwrap_or(0));
                    slice(page, at + 1, cb)
                        .map(CharProps::parse)
                        .unwrap_or_default()
                }
            };
            spans.push((start, end, props));
        }
    }
    sorted(spans)
}

/// What a paragraph's properties say about its mark.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ParagraphProps {
    pub style: u16,
    /// The mark ends a table row and holds no text.
    pub row_end: bool,
}

/// Paragraph properties by stream offset, from the PapxFkp pages.
pub fn paragraph_spans(plc: &[u8], word: &[u8]) -> Spans<ParagraphProps> {
    let mut spans = Vec::new();
    for pn in pages(plc) {
        let Some(page) = pn.checked_mul(512).and_then(|at| slice(word, at, 512)) else {
            continue;
        };
        let paragraphs = usize::from(page[511]);
        for i in 0..paragraphs {
            let (Some(start), Some(end)) = (u32_at(page, i * 4), u32_at(page, i * 4 + 4)) else {
                break;
            };
            // BxPap is 13 bytes; its first is the offset of the PapxInFkp, in words.
            let at = usize::from(u8_at(page, 4 * (paragraphs + 1) + i * 13).unwrap_or(0)) * 2;
            // GrpPrlAndIstd: 2*cb - 1 bytes after cb, or 2*cb' after a 0 and cb'.
            let papx = match (at, u8_at(page, at)) {
                (0, _) | (_, None) => None,
                (_, Some(0)) => {
                    u8_at(page, at + 1).and_then(|cb| slice(page, at + 2, usize::from(cb) * 2))
                }
                (_, Some(cb)) => slice(page, at + 1, usize::from(cb) * 2 - 1),
            };
            let props = papx.map_or_else(ParagraphProps::default, |papx| ParagraphProps {
                style: u16_at(papx, 0).unwrap_or(0),
                row_end: ends_row(papx.get(2..).unwrap_or_default()),
            });
            spans.push((start, end, props));
        }
    }
    sorted(spans)
}

fn sorted<T>(mut spans: Vec<(u32, u32, T)>) -> Spans<T> {
    spans.sort_by_key(|s| s.0);
    Spans { spans }
}

/// The stylesheet ([MS-DOC] 2.9.271): each style's character properties, with
/// those of its base styles under them.
#[derive(Debug, Default)]
pub struct Styles {
    /// The fonts a style that sets none uses (rgftcStandardChpStsh).
    pub defaults: CharProps,
    resolved: Vec<CharProps>,
}

const NO_STYLE: u16 = 0x0FFF;

impl Styles {
    pub fn parse(stsh: &[u8]) -> Styles {
        let cb_stshi = usize::from(u16_at(stsh, 0).unwrap_or(0));
        let stshi = slice(stsh, 2, cb_stshi).unwrap_or_default();
        let count = usize::from(u16_at(stshi, 0).unwrap_or(0));
        let base_size = usize::from(u16_at(stshi, 2).unwrap_or(10));
        let defaults = CharProps {
            ascii: u16_at(stshi, 12),
            other: u16_at(stshi, 16),
            ..CharProps::default()
        };
        let mut own = Vec::new();
        let mut at = 2 + cb_stshi;
        for _ in 0..count {
            let Some(cb) = u16_at(stsh, at).map(usize::from) else {
                break;
            };
            let std = slice(stsh, at + 2, cb).unwrap_or_default();
            own.push(style(std, base_size));
            at += 2 + cb;
        }
        let mut styles = Styles {
            defaults,
            resolved: Vec::new(),
        };
        styles.resolved = (0..own.len()).map(|i| resolve(&own, i, 0)).collect();
        styles
    }

    /// The character properties of style `istd`, base styles included.
    pub fn props(&self, istd: u16) -> CharProps {
        self.resolved
            .get(usize::from(istd))
            .copied()
            .unwrap_or_default()
    }
}

/// A style's base and its own character properties.
fn style(std: &[u8], base_size: usize) -> Option<(u16, CharProps)> {
    let kind = u16_at(std, 2)? & 0x000F;
    let base = u16_at(std, 2)? >> 4;
    let upx_count = usize::from(u16_at(std, 4)? & 0x000F);
    // The character properties are the last UPX of paragraph, character and table styles.
    let chpx_index = match kind {
        1 => 1,
        2 => 0,
        3 => 2,
        _ => return Some((base, CharProps::default())),
    };
    let name_len = usize::from(u16_at(std, base_size)?);
    let mut at = base_size + 2 + name_len * 2 + 2;
    for i in 0..upx_count {
        let cb = usize::from(u16_at(std, at)?);
        if i == chpx_index {
            return Some((base, CharProps::parse(slice(std, at + 2, cb)?)));
        }
        at += 2 + cb + cb % 2;
    }
    Some((base, CharProps::default()))
}

/// Style `i`'s properties over those of its bases, up to 11 levels deep.
fn resolve(own: &[Option<(u16, CharProps)>], i: usize, depth: usize) -> CharProps {
    let Some(Some((base, props))) = own.get(i) else {
        return CharProps::default();
    };
    let under = if *base == NO_STYLE || usize::from(*base) == i || depth > 10 {
        CharProps::default()
    } else {
        resolve(own, usize::from(*base), depth + 1)
    };
    under.and(*props)
}

/// Font names from the SttbfFfn ([MS-DOC] 2.9.284); the index is the font number.
pub fn fonts(sttb: &[u8]) -> Vec<Font> {
    let count = usize::from(u16_at(sttb, 0).unwrap_or(0));
    let extra = usize::from(u16_at(sttb, 2).unwrap_or(0));
    let mut out = Vec::new();
    let mut at = 4;
    for _ in 0..count {
        let Some(cb) = u8_at(sttb, at).map(usize::from) else {
            break;
        };
        // The FFN starts with this length byte; its name, xszFfn, starts at byte 40.
        let ffn = slice(sttb, at, cb + 1).unwrap_or_default();
        let units: Vec<u16> = ffn
            .get(40..)
            .unwrap_or_default()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|&p| u16::from_le_bytes(p))
            .take_while(|&u| u != 0)
            .collect();
        out.push(Font {
            name: String::from_utf16_lossy(&units),
        });
        at += cb + 1 + extra;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spans_give_the_default_outside_and_at_their_end() {
        let spans = sorted(vec![(10, 20, 2u16), (0, 5, 1)]);
        assert_eq!(spans.at(0), 1);
        assert_eq!(spans.at(5), 0);
        assert_eq!(spans.at(19), 2);
        assert_eq!(spans.at(20), 0);
    }

    #[test]
    fn styles_inherit_and_stop_at_loops() {
        let p = |ascii| CharProps {
            ascii: Some(ascii),
            ..CharProps::default()
        };
        let own = vec![
            Some((NO_STYLE, p(1))),
            Some((0, CharProps::default())),
            Some((3, p(2))),
            Some((2, CharProps::default())),
        ];
        assert_eq!(resolve(&own, 1, 0).ascii, Some(1));
        assert_eq!(resolve(&own, 3, 0).ascii, Some(2));
    }
}
