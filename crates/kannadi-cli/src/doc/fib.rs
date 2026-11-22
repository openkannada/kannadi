//! The File Information Block at the start of the WordDocument stream
//! ([MS-DOC] 2.5.1): where each table is, and how long each story is.

use anyhow::{bail, Result};

use super::bytes::{u16_at, u32_at};

/// Index in [`STORIES`] of the headers and footers.
pub const HEADERS: usize = 2;

/// The stories in the order their text follows the main document ([MS-DOC] 2.4.1),
/// with their index in FibRgLw97.
pub const STORIES: [(&str, usize); 7] = [
    ("body", 3),
    ("footnotes", 4),
    ("headers and footers", 5),
    ("comments", 7),
    ("endnotes", 8),
    ("text boxes", 9),
    ("header text boxes", 10),
];

#[derive(Debug)]
pub struct Fib {
    /// The table stream is 1Table, else 0Table.
    pub table_1: bool,
    /// Characters in each story of [`STORIES`].
    pub ccp: [u32; 7],
    pub stsh: (u32, u32),
    /// Where each header, footer and note separator starts in its story.
    pub hdd: (u32, u32),
    pub chpx: (u32, u32),
    pub papx: (u32, u32),
    pub fonts: (u32, u32),
    pub clx: (u32, u32),
}

// Indices of (fc, lcb) pairs in FibRgFcLcb97.
const STSHF: usize = 1;
const PLCF_HDD: usize = 11;
const PLCF_BTE_CHPX: usize = 12;
const PLCF_BTE_PAPX: usize = 13;
const STTBF_FFN: usize = 15;
const CLX: usize = 33;

impl Fib {
    pub fn parse(word: &[u8]) -> Result<Fib> {
        if u16_at(word, 0) != Some(0xA5EC) {
            bail!("not a Word document");
        }
        if word.len() < 34 {
            bail!("the document's information block is too short");
        }
        let n_fib = u16_at(word, 2).unwrap_or(0);
        let flags = u16_at(word, 0x0A).unwrap_or(0);
        if flags & 0x0100 != 0 {
            bail!("the document is password-protected");
        }
        // Word 97 wrote nFib 0x00C1; Word 6 and Word 95 wrote 0x0065 to 0x0068.
        if n_fib < 0x00C1 {
            bail!(
                "a Word 6 or Word 95 document (nFib {n_fib:#06X}); only Word 97 and later are read"
            );
        }
        let csw = usize::from(u16_at(word, 32).unwrap_or(0));
        let at_lw = 34 + csw * 2;
        let cslw = usize::from(u16_at(word, at_lw).unwrap_or(0));
        let lw = at_lw + 2;
        let at_fc = lw + cslw * 4;
        let pairs = usize::from(u16_at(word, at_fc).unwrap_or(0));
        let fc = at_fc + 2;
        if cslw < 11 || pairs <= CLX {
            bail!("the document's information block is too short");
        }
        let pair = |i: usize| -> Result<(u32, u32)> {
            match (u32_at(word, fc + i * 8), u32_at(word, fc + i * 8 + 4)) {
                (Some(fc), Some(lcb)) => Ok((fc, lcb)),
                _ => bail!("the document's information block is cut short"),
            }
        };
        let mut ccp = [0; 7];
        for (slot, (_, index)) in ccp.iter_mut().zip(STORIES) {
            *slot = u32_at(word, lw + index * 4).unwrap_or(0);
        }
        Ok(Fib {
            table_1: flags & 0x0200 != 0,
            ccp,
            stsh: pair(STSHF)?,
            hdd: pair(PLCF_HDD)?,
            chpx: pair(PLCF_BTE_CHPX)?,
            papx: pair(PLCF_BTE_PAPX)?,
            fonts: pair(STTBF_FFN)?,
            clx: pair(CLX)?,
        })
    }
}
