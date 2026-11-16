//! ToUnicode character maps. lopdf rejects some maps that PDF makers write, such
//! as a `CMapName` with 2 slashes, so the `bfchar` and `bfrange` entries are read
//! here.

use std::collections::HashMap;

/// Strings by their code, where codes are 1 to 4 bytes long.
#[derive(Debug, Default)]
pub struct CMap {
    map: HashMap<Vec<u8>, String>,
    lengths: Vec<usize>,
}

/// The most codes 1 `bfrange` may cover; codes are at most 2 bytes in practice.
const MAX_RANGE: u32 = 0x1_0000;

impl CMap {
    pub fn parse(source: &[u8]) -> Self {
        let text = String::from_utf8_lossy(source);
        let tokens = tokens(&text);
        let mut cmap = CMap::default();
        let mut i = 0;
        while i < tokens.len() {
            match tokens[i] {
                Token::Word("beginbfchar") => {
                    i += 1;
                    while let (Some(Token::Hex(src)), Some(Token::Hex(dst))) =
                        (tokens.get(i), tokens.get(i + 1))
                    {
                        cmap.insert(src.clone(), utf16(dst));
                        i += 2;
                    }
                }
                Token::Word("beginbfrange") => {
                    i += 1;
                    while let (Some(Token::Hex(lo)), Some(Token::Hex(hi))) =
                        (tokens.get(i), tokens.get(i + 1))
                    {
                        match tokens.get(i + 2) {
                            Some(Token::Hex(dst)) => cmap.range(lo, hi, |k| {
                                let mut d = dst.clone();
                                add(&mut d, k);
                                Some(utf16(&d))
                            }),
                            Some(Token::Array(list)) => {
                                cmap.range(lo, hi, |k| list.get(k as usize).map(|d| utf16(d)))
                            }
                            _ => break,
                        }
                        i += 3;
                    }
                }
                _ => i += 1,
            }
        }
        cmap.lengths.sort_unstable();
        cmap.lengths.dedup();
        cmap
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// The text of a shown string, reading the longest known code at each point.
    /// Bytes with no code are read as Windows-1252.
    pub fn decode(&self, bytes: &[u8]) -> String {
        let mut out = String::new();
        let mut i = 0;
        while i < bytes.len() {
            let hit = self.lengths.iter().rev().find_map(|&n| {
                let code = bytes.get(i..i + n)?;
                self.map.get(code).map(|s| (s, n))
            });
            match hit {
                Some((s, n)) => {
                    out.push_str(s);
                    i += n;
                }
                None => {
                    out.push_str(&kannadi::legacy::decode_windows_1252(&bytes[i..=i]));
                    i += 1;
                }
            }
        }
        out
    }

    fn insert(&mut self, code: Vec<u8>, text: String) {
        if (1..=4).contains(&code.len()) {
            self.lengths.push(code.len());
            self.map.insert(code, text);
        }
    }

    fn range(&mut self, lo: &[u8], hi: &[u8], dst: impl Fn(u32) -> Option<String>) {
        let (Some(start), Some(end)) = (number(lo), number(hi)) else {
            return;
        };
        for k in 0..=end.saturating_sub(start).min(MAX_RANGE - 1) {
            let Some(text) = dst(k) else { break };
            let mut code = lo.to_vec();
            add(&mut code, k);
            self.insert(code, text);
        }
    }
}

#[derive(Debug)]
enum Token<'a> {
    Hex(Vec<u8>),
    Array(Vec<Vec<u8>>),
    Word(&'a str),
}

fn tokens(text: &str) -> Vec<Token<'_>> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(c) = rest.chars().next() {
        if c == '<' && !rest.starts_with("<<") {
            let end = rest.find('>').unwrap_or(rest.len());
            out.push(Token::Hex(hex(&rest[1..end])));
            rest = &rest[(end + 1).min(rest.len())..];
        } else if c == '[' {
            let end = rest.find(']').unwrap_or(rest.len());
            let items = rest[1..end]
                .split('<')
                .filter_map(|p| p.split_once('>'))
                .map(|(h, _)| hex(h))
                .collect();
            out.push(Token::Array(items));
            rest = &rest[(end + 1).min(rest.len())..];
        } else if c.is_ascii_alphabetic() {
            let end = rest
                .find(|c: char| !c.is_ascii_alphanumeric())
                .unwrap_or(rest.len());
            out.push(Token::Word(&rest[..end]));
            rest = &rest[end..];
        } else {
            rest = &rest[c.len_utf8()..];
        }
    }
    out
}

fn hex(text: &str) -> Vec<u8> {
    let digits: Vec<u8> = text.bytes().filter(u8::is_ascii_hexdigit).collect();
    digits
        .chunks(2)
        .map(|pair| {
            let s = std::str::from_utf8(pair).unwrap_or("0");
            u8::from_str_radix(s, 16).unwrap_or(0) << if pair.len() == 1 { 4 } else { 0 }
        })
        .collect()
}

fn number(bytes: &[u8]) -> Option<u32> {
    (bytes.len() <= 4).then(|| bytes.iter().fold(0u32, |n, &b| n << 8 | u32::from(b)))
}

/// Adds `k` to the big-endian number in `bytes`, keeping its length.
fn add(bytes: &mut [u8], k: u32) {
    let mut carry = k;
    for b in bytes.iter_mut().rev() {
        let sum = u32::from(*b) + (carry & 0xFF);
        *b = (sum & 0xFF) as u8;
        carry = (carry >> 8) + (sum >> 8);
    }
}

fn utf16(bytes: &[u8]) -> String {
    let units: Vec<u16> = bytes
        .chunks(2)
        .map(|c| u16::from(c[0]) << 8 | u16::from(*c.get(1).unwrap_or(&0)))
        .collect();
    String::from_utf16_lossy(&units)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bfchar_and_bfrange_entries_are_read() {
        let source = b"/CMapName //AQKYTL+Nudi01k def\n\
            1 begincodespacerange <00> <FF> endcodespacerange\n\
            2 beginbfchar <29> <0043> <2A> <0050> endbfchar\n\
            2 beginbfrange <2C> <2D> <00C0> <30> <31> [<00A3> <00DF>] endbfrange";
        let cmap = CMap::parse(source);
        assert_eq!(cmap.decode(b")*,-01\xFF"), "CPÀÁ£ßÿ");
    }

    #[test]
    fn huge_or_broken_ranges_are_safe() {
        let cmap =
            CMap::parse(b"beginbfrange <00000000> <FFFFFFFF> <0041> endbfrange beginbfchar <0001");
        assert_eq!(cmap.decode(b"\x00\x00\x00\x00"), "A");
        assert!(CMap::parse(b"").is_empty());
    }
}
