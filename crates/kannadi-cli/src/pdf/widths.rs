//! Glyph advances, for telling word gaps from glyphs drawn in pieces.

use lopdf::{Dictionary, Document, Object};

use super::resolve;

/// Glyph advances in thousandths of the font size.
#[derive(Debug)]
pub enum Widths {
    Simple {
        first: u32,
        widths: Vec<f32>,
    },
    /// Ranges of CIDs with 1 width each, sorted and not overlapping.
    Cid {
        ranges: Vec<(u32, u32, f32)>,
        default: f32,
    },
}

impl Widths {
    pub fn of<'a>(doc: &'a Document, font: &'a Dictionary) -> Self {
        let entry = |dict: &'a Dictionary, key: &[u8]| -> Option<&'a Object> {
            resolve(doc, dict.get(key).ok()?)
        };
        let subtype = font.get(b"Subtype").and_then(Object::as_name).ok();
        if subtype != Some(b"Type0".as_slice()) {
            let first = font.get(b"FirstChar").ok().and_then(number).unwrap_or(0.0);
            let widths = entry(font, b"Widths")
                .and_then(|o| o.as_array().ok())
                .map(|a| a.iter().filter_map(number).collect())
                .unwrap_or_default();
            return Widths::Simple {
                first: first.clamp(0.0, 255.0) as u32,
                widths,
            };
        }
        let descendant = entry(font, b"DescendantFonts")
            .and_then(|o| o.as_array().ok()?.first())
            .and_then(|o| resolve(doc, o)?.as_dict().ok());
        let default = descendant
            .and_then(|d| d.get(b"DW").ok())
            .and_then(number)
            .unwrap_or(1000.0);
        let list = descendant
            .and_then(|d| entry(d, b"W"))
            .and_then(|o| o.as_array().ok())
            .map_or(&[][..], Vec::as_slice);
        Widths::Cid {
            ranges: cid_ranges(list, |o| resolve(doc, o)),
            default,
        }
    }

    /// The advance of a shown string, in thousandths of the font size.
    pub fn advance(&self, bytes: &[u8]) -> f32 {
        match self {
            Widths::Simple { first, widths } => bytes
                .iter()
                .map(|&b| {
                    let i = u32::from(b).checked_sub(*first).map(|i| i as usize);
                    i.and_then(|i| widths.get(i)).copied().unwrap_or(500.0)
                })
                .sum(),
            Widths::Cid { ranges, default } => bytes
                .chunks(2)
                .map(|c| {
                    let cid = u32::from(c[0]) << 8 | u32::from(*c.get(1).unwrap_or(&0));
                    let i = ranges.partition_point(|&(_, end, _)| end < cid);
                    match ranges.get(i) {
                        Some(&(start, _, w)) if start <= cid => w,
                        _ => *default,
                    }
                })
                .sum(),
        }
    }
}

/// The `W` array as ranges. CIDs are 2 bytes, so anything above 0xFFFF is dropped.
fn cid_ranges<'a>(
    list: &'a [Object],
    resolve: impl Fn(&'a Object) -> Option<&'a Object>,
) -> Vec<(u32, u32, f32)> {
    let cid = |o: &Object| {
        number(o)
            .filter(|n| (0.0..=65_535.0).contains(n))
            .map(|n| n as u32)
    };
    let mut ranges = Vec::new();
    let mut i = 0;
    while i + 1 < list.len() {
        let Some(start) = cid(&list[i]) else { break };
        match resolve(&list[i + 1]) {
            Some(Object::Array(run)) => {
                for (k, w) in run.iter().filter_map(number).enumerate() {
                    let c = start as usize + k;
                    if c > 0xFFFF {
                        break;
                    }
                    ranges.push((c as u32, c as u32, w));
                }
                i += 2;
            }
            _ => {
                let (Some(end), Some(w)) = (cid(&list[i + 1]), list.get(i + 2).and_then(number))
                else {
                    break;
                };
                if start <= end {
                    ranges.push((start, end, w));
                }
                i += 3;
            }
        }
    }
    ranges.sort_by_key(|&(start, _, _)| start);
    ranges.dedup_by(|later, earlier| later.0 <= earlier.1);
    ranges
}

fn number(o: &Object) -> Option<f32> {
    o.as_float().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cid_ranges_cover_runs_and_spans() {
        let list = vec![
            Object::Integer(1),
            Object::Array(vec![Object::Integer(250), Object::Integer(300)]),
            Object::Integer(10),
            Object::Integer(20),
            Object::Integer(600),
            Object::Integer(4_294_967_295),
            Object::Integer(4_294_967_295),
            Object::Integer(9),
        ];
        let widths = Widths::Cid {
            ranges: cid_ranges(&list, Some),
            default: 1000.0,
        };
        assert_eq!(
            widths.advance(&[0, 1, 0, 2, 0, 15, 0, 30]),
            250.0 + 300.0 + 600.0 + 1000.0
        );
    }
}
