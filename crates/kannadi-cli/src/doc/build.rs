//! Writes small Word 97 documents for tests: a FIB, the text in pieces, 1 property
//! page per 20 runs, 1 paragraph page, a stylesheet and a font table.

use std::io::{Cursor, Write};

use kannadi::legacy::windows_1252_byte;

pub const FONT_ASCII: [u8; 2] = [0x4F, 0x4A];
pub const FONT_OTHER: [u8; 2] = [0x51, 0x4A];
pub const CHAR_STYLE: [u8; 2] = [0x30, 0x4A];

/// Property modifiers that set both Latin font slots to `ftc`.
pub fn font(ftc: u16) -> Vec<u8> {
    let [lo, hi] = ftc.to_le_bytes();
    [FONT_ASCII, [lo, hi], FONT_OTHER, [lo, hi]].concat()
}

pub fn char_style(istd: u16) -> Vec<u8> {
    [CHAR_STYLE, istd.to_le_bytes()].concat()
}

pub struct Style {
    /// 1 for a paragraph style, 2 for a character style.
    pub kind: u16,
    pub base: u16,
    pub grpprl: Vec<u8>,
}

pub struct Piece {
    pub compressed: bool,
    /// Property modifiers of the piece itself, stored in a Prc.
    pub grpprl: Option<Vec<u8>>,
    /// Each run's property modifiers and text.
    pub runs: Vec<(Vec<u8>, String)>,
}

impl Piece {
    pub fn new(compressed: bool, runs: Vec<(Vec<u8>, &str)>) -> Self {
        Piece {
            compressed,
            grpprl: None,
            runs: runs.into_iter().map(|(g, t)| (g, t.to_string())).collect(),
        }
    }
}

#[derive(Default)]
pub struct Doc {
    pub fonts: Vec<&'static str>,
    pub styles: Vec<Style>,
    /// The fonts styles fall back to (rgftcStandardChpStsh), ASCII and other.
    pub default_fonts: (u16, u16),
    pub paragraph_style: u16,
    pub pieces: Vec<Piece>,
    /// Characters per story; when all zero, all text is in the body.
    pub ccp: [u32; 7],
    pub n_fib: Option<u16>,
    pub encrypted: bool,
    /// Property modifiers after the style of each paragraph, in order; paragraphs
    /// past the end have none.
    pub paragraphs: Vec<Vec<u8>>,
    /// Bytes in each style's Stdf: 10, or 18 as Word 2000 and later write.
    pub std_base_size: Option<u16>,
    /// The PlcfHdd: where each header part starts in the header story.
    pub hdd: Vec<u32>,
}

fn put16(b: &mut [u8], at: usize, v: u16) {
    b[at..at + 2].copy_from_slice(&v.to_le_bytes());
}

fn put32(b: &mut [u8], at: usize, v: u32) {
    b[at..at + 4].copy_from_slice(&v.to_le_bytes());
}

fn pad_to_page(b: &mut Vec<u8>) {
    b.resize(b.len().div_ceil(512) * 512, 0);
}

impl Doc {
    pub fn bytes(&self) -> Vec<u8> {
        let mut word = vec![0u8; 1024];
        let mut table = Vec::new();
        let mut runs: Vec<(u32, u32, Vec<u8>)> = Vec::new();
        let mut plc_cps = vec![0u32];
        let mut pcds = Vec::new();
        let mut prcs = Vec::new();
        let mut cp = 0u32;
        let mut marks = Vec::new();
        for piece in &self.pieces {
            let offset = word.len() as u32;
            for (grpprl, text) in &piece.runs {
                let start = word.len() as u32;
                for c in text.chars() {
                    if piece.compressed {
                        word.push(windows_1252_byte(c).expect("a Windows-1252 character"));
                    } else {
                        let mut units = [0u16; 2];
                        for u in c.encode_utf16(&mut units) {
                            word.extend(u.to_le_bytes());
                        }
                    }
                    cp += c.len_utf16() as u32;
                    if c == '\r' || c == '\u{07}' {
                        marks.push(word.len() as u32);
                    }
                }
                runs.push((start, word.len() as u32, grpprl.clone()));
            }
            plc_cps.push(cp);
            let fc = if piece.compressed {
                (offset * 2) | 0x4000_0000
            } else {
                offset
            };
            let prm = match &piece.grpprl {
                Some(g) => {
                    prcs.push(g.clone());
                    (((prcs.len() - 1) as u16) << 1) | 1
                }
                None => 0,
            };
            let mut pcd = vec![0u8; 8];
            put32(&mut pcd, 2, fc);
            put16(&mut pcd, 6, prm);
            pcds.extend(pcd);
        }
        let text = (1024u32, word.len() as u32);
        pad_to_page(&mut word);

        let mut chpx_plc_fcs = Vec::new();
        let mut chpx_pns = Vec::new();
        for chunk in runs.chunks(20) {
            let mut page = vec![0u8; 512];
            let n = chunk.len();
            let mut free = 510;
            for (i, (start, end, grpprl)) in chunk.iter().enumerate() {
                put32(&mut page, i * 4, *start);
                put32(&mut page, i * 4 + 4, *end);
                if !grpprl.is_empty() {
                    let at = (free - 1 - grpprl.len()) & !1;
                    page[at] = grpprl.len() as u8;
                    page[at + 1..at + 1 + grpprl.len()].copy_from_slice(grpprl);
                    page[4 * (n + 1) + i] = (at / 2) as u8;
                    free = at;
                }
            }
            page[511] = n as u8;
            chpx_plc_fcs.push(chunk[0].0);
            chpx_pns.push((word.len() / 512) as u32);
            word.extend(page);
        }
        chpx_plc_fcs.push(text.1);

        let papx_pn = (word.len() / 512) as u32;
        word.extend(self.paragraph_page(text, &marks));

        let place = |table: &mut Vec<u8>, data: &[u8]| {
            let at = table.len() as u32;
            table.extend_from_slice(data);
            (at, data.len() as u32)
        };
        let stsh = place(&mut table, &self.stylesheet());
        let fonts = place(&mut table, &self.font_table());
        let mut clx = Vec::new();
        for g in &prcs {
            clx.push(0x01);
            clx.extend((g.len() as u16).to_le_bytes());
            clx.extend(g);
        }
        let mut plc: Vec<u8> = plc_cps.iter().flat_map(|c| c.to_le_bytes()).collect();
        plc.extend(&pcds);
        clx.push(0x02);
        clx.extend((plc.len() as u32).to_le_bytes());
        clx.extend(&plc);
        let clx = place(&mut table, &clx);
        let mut chpx_plc: Vec<u8> = chpx_plc_fcs.iter().flat_map(|c| c.to_le_bytes()).collect();
        chpx_plc.extend(chpx_pns.iter().flat_map(|c| c.to_le_bytes()));
        let chpx = place(&mut table, &chpx_plc);
        let papx_plc: Vec<u8> = [text.0, text.1, papx_pn]
            .iter()
            .flat_map(|c| c.to_le_bytes())
            .collect();
        let papx = place(&mut table, &papx_plc);

        put16(&mut word, 0, 0xA5EC);
        put16(&mut word, 2, self.n_fib.unwrap_or(0x00C1));
        let flags = 0x0200 | if self.encrypted { 0x0100 } else { 0 };
        put16(&mut word, 0x0A, flags);
        put16(&mut word, 32, 14);
        put16(&mut word, 62, 22);
        let ccp = if self.ccp == [0; 7] {
            [cp, 0, 0, 0, 0, 0, 0]
        } else {
            self.ccp
        };
        for (i, index) in [3, 4, 5, 7, 8, 9, 10].into_iter().enumerate() {
            put32(&mut word, 64 + index * 4, ccp[i]);
        }
        put16(&mut word, 152, 93);
        let hdd: Vec<u8> = self.hdd.iter().flat_map(|c| c.to_le_bytes()).collect();
        let hdd = place(&mut table, &hdd);
        let pairs = [
            (1, stsh),
            (11, hdd),
            (12, chpx),
            (13, papx),
            (15, fonts),
            (33, clx),
        ];
        for (i, (fc, lcb)) in pairs {
            put32(&mut word, 154 + i * 8, fc);
            put32(&mut word, 154 + i * 8 + 4, lcb);
        }
        compound(&[("/WordDocument", &word), ("/1Table", &table)])
    }

    /// 1 PapxFkp page: a paragraph ends after each mark in `marks`, and its
    /// GrpPrlAndIstd is stored in the short form when its length is odd and in
    /// the long form when it is even, as Word does.
    fn paragraph_page(&self, text: (u32, u32), marks: &[u32]) -> Vec<u8> {
        let mut ends = marks.to_vec();
        if ends.last() != Some(&text.1) {
            ends.push(text.1);
        }
        let n = ends.len();
        let mut page = vec![0u8; 512];
        let mut start = text.0;
        let mut free = 511;
        for (i, &end) in ends.iter().enumerate() {
            put32(&mut page, i * 4, start);
            put32(&mut page, i * 4 + 4, end);
            start = end;
            let mut papx = self.paragraph_style.to_le_bytes().to_vec();
            papx.extend(self.paragraphs.get(i).cloned().unwrap_or_default());
            let stored = if papx.len() % 2 == 1 {
                [vec![papx.len().div_ceil(2) as u8], papx].concat()
            } else {
                [vec![0, (papx.len() / 2) as u8], papx].concat()
            };
            let at = (free - stored.len()) & !1;
            page[at..at + stored.len()].copy_from_slice(&stored);
            page[4 * (n + 1) + i * 13] = (at / 2) as u8;
            free = at;
        }
        page[511] = n as u8;
        page
    }

    fn stylesheet(&self) -> Vec<u8> {
        let base = self.std_base_size.unwrap_or(10);
        let mut stshi = vec![0u8; 18];
        put16(&mut stshi, 0, self.styles.len() as u16);
        put16(&mut stshi, 2, base);
        put16(&mut stshi, 12, self.default_fonts.0);
        put16(&mut stshi, 16, self.default_fonts.1);
        let mut out = (stshi.len() as u16).to_le_bytes().to_vec();
        out.extend(stshi);
        for style in &self.styles {
            let mut std = vec![0u8; usize::from(base)];
            put16(&mut std, 2, style.kind | (style.base << 4));
            let upx = if style.kind == 1 { 2 } else { 1 };
            put16(&mut std, 4, upx);
            std.extend([1, 0, b'S', 0, 0, 0]);
            if style.kind == 1 {
                std.extend([2, 0, 0, 0]);
            }
            std.extend((style.grpprl.len() as u16).to_le_bytes());
            std.extend(&style.grpprl);
            if style.grpprl.len() % 2 == 1 {
                std.push(0);
            }
            out.extend((std.len() as u16).to_le_bytes());
            out.extend(std);
        }
        out
    }

    fn font_table(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend((self.fonts.len() as u16).to_le_bytes());
        out.extend([0, 0]);
        for name in &self.fonts {
            let mut ffn = vec![0u8; 40];
            ffn.extend(name.encode_utf16().chain([0]).flat_map(u16::to_le_bytes));
            ffn[0] = (ffn.len() - 1) as u8;
            out.extend(ffn);
        }
        out
    }
}

/// An OLE compound file holding `streams`.
pub fn compound(streams: &[(&str, &[u8])]) -> Vec<u8> {
    let mut file = cfb::CompoundFile::create(Cursor::new(Vec::new())).unwrap();
    for (name, data) in streams {
        file.create_stream(name).unwrap().write_all(data).unwrap();
    }
    file.flush().unwrap();
    file.into_inner().into_inner()
}
