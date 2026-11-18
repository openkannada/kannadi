//! ToUnicode character maps. lopdf rejects some maps that PDF makers write, such
//! as a `CMapName` with 2 slashes, so the `bfchar` and `bfrange` entries are read
//! here.

use std::collections::HashMap;

/// Strings by their code, where codes are 1 to 4 bytes long.
#[derive(Debug, Default)]
pub struct CMap {
    map: HashMap<Vec<u8>, String>,
    /// Whether some code is `n + 1` bytes long, by `n`.
    lengths: [bool; 4],
    /// How many more entries the map may take.
    room: usize,
    /// The `codespacerange` entries: the lowest and highest code of each.
    spaces: Vec<(Vec<u8>, Vec<u8>)>,
}

/// The most codes 1 `bfrange` may cover; codes are at most 2 bytes in practice.
const MAX_RANGE: u32 = 0x1_0000;

/// The most entries a map may take in all, counting repeats, so a small hostile
/// map cannot take gigabytes or minutes.
const MAX_ENTRIES: usize = 0x2_0000;

impl CMap {
    pub fn parse(source: &[u8]) -> Self {
        let text = String::from_utf8_lossy(source);
        let tokens = tokens(&text);
        let mut cmap = CMap {
            room: MAX_ENTRIES,
            ..CMap::default()
        };
        let mut i = 0;
        while i < tokens.len() {
            match tokens[i] {
                Token::Word("begincodespacerange") => {
                    i += 1;
                    while let (Some(Token::Hex(lo)), Some(Token::Hex(hi))) =
                        (tokens.get(i), tokens.get(i + 1))
                    {
                        if lo.len() == hi.len() && (1..=4).contains(&lo.len()) {
                            cmap.spaces.push((lo.clone(), hi.clone()));
                        }
                        i += 2;
                    }
                }
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
        cmap
    }

    /// The text of 1 code.
    pub fn get(&self, code: &[u8]) -> Option<&str> {
        self.map.get(code).map(String::as_str)
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
            let hit = (1..=4).rev().filter(|n| self.lengths[n - 1]).find_map(|n| {
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

    /// The text of a shown string in a font whose text is Unicode, and how many
    /// codes have no entry. Each of those becomes U+FFFD, so lost text shows.
    pub fn read(&self, bytes: &[u8]) -> (String, usize) {
        let longest = (1..=4).rev().find(|n| self.lengths[n - 1]).unwrap_or(1);
        let mut out = String::new();
        let mut missing = 0;
        let mut i = 0;
        while i < bytes.len() {
            let hit = (1..=4).rev().filter(|n| self.lengths[n - 1]).find_map(|n| {
                let code = bytes.get(i..i + n)?;
                self.map.get(code).map(|s| (s, n))
            });
            match hit {
                Some((s, n)) => {
                    out.push_str(s);
                    i += n;
                }
                None => {
                    out.push('\u{FFFD}');
                    missing += 1;
                    i += self.code_length(&bytes[i..]).unwrap_or(longest);
                }
            }
        }
        (out, missing)
    }

    /// The length of the code at the start of `bytes`, by the codespace ranges.
    fn code_length(&self, bytes: &[u8]) -> Option<usize> {
        self.spaces.iter().find_map(|(lo, hi)| {
            let code = bytes.get(..lo.len())?;
            let inside = code
                .iter()
                .zip(lo.iter().zip(hi))
                .all(|(c, (l, h))| (l..=h).contains(&c));
            inside.then_some(lo.len())
        })
    }

    /// Adds an entry, and says whether the map has room for more.
    fn insert(&mut self, code: Vec<u8>, text: String) -> bool {
        if self.room == 0 {
            return false;
        }
        if (1..=4).contains(&code.len()) {
            self.room -= 1;
            self.lengths[code.len() - 1] = true;
            self.map.insert(code, text);
        }
        true
    }

    fn range(&mut self, lo: &[u8], hi: &[u8], dst: impl Fn(u32) -> Option<String>) {
        let (Some(start), Some(end)) = (number(lo), number(hi)) else {
            return;
        };
        for k in 0..=end.saturating_sub(start).min(MAX_RANGE - 1) {
            let Some(text) = dst(k) else { break };
            let mut code = lo.to_vec();
            add(&mut code, k);
            if !self.insert(code, text) {
                break;
            }
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
        if rest.starts_with("<<") || rest.starts_with(">>") {
            rest = &rest[2..];
        } else if c == '<' {
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
    fn a_missing_code_is_as_long_as_its_codespace_says() {
        let map = CMap::parse(
            b"2 begincodespacerange <00> <7F> <8000> <FFFF> endcodespacerange \
            2 beginbfchar <41> <0CA8> <8001> <0CAE> endbfchar",
        );
        assert_eq!(
            map.read(&[0x10, 0x41, 0x80, 0x02, 0x80, 0x01]),
            ("\u{FFFD}ನ\u{FFFD}ಮ".to_string(), 2)
        );
    }

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

    #[test]
    fn a_map_takes_a_bounded_number_of_entries() {
        let source = b"beginbfrange <00010000> <0001FFFF> <0041> <00020000> <0002FFFF> <0041> \
            <00030000> <0003FFFF> <0041> endbfrange";
        assert_eq!(CMap::parse(source).map.len(), MAX_ENTRIES);
    }

    #[test]
    fn dictionaries_are_not_read_as_codes() {
        let cmap = CMap::parse(b"<< /Registry (Adobe) >> beginbfchar <41> <0042> endbfchar");
        assert_eq!(cmap.decode(b"A"), "B");
        let found = tokens("<</A <0041>>>");
        let hex: Vec<&Vec<u8>> = found
            .iter()
            .filter_map(|t| match t {
                Token::Hex(h) => Some(h),
                _ => None,
            })
            .collect();
        assert_eq!(hex, [&vec![0x00, 0x41]], "{found:?}");
    }
}
