//! Legacy Kannada font text to and from Unicode.
//!
//! Before Unicode, Kannada was typed in fonts that draw Kannada shapes at Latin-1
//! code points. Each family of fonts has its own glyph layout, a [`Layout`]. Nudi
//! (up to version 5) and Baraha's ANSI fonts (BRH Kannada and others) share 1.
//! Text in those fonts reads as `PÀ£ÀßqÀ` until it is converted.
//!
//! Glyphs are stored in drawing order: a subscript consonant comes after its
//! syllable's vowel sign, the arkavattu after the cluster, and some vowel signs
//! are split into 2 or 3 glyphs. [`to_unicode`] rebuilds each syllable in
//! Unicode order. It reports every glyph it cannot place and every syllable that
//! is not valid; spacing glyphs, which carry no text, are the only glyphs dropped.
//! [`from_unicode`] goes the other way, for documents that must still be printed
//! in a legacy font, and reports every character it cannot encode.

mod decode;
mod encode;
mod table;

pub use decode::{
    nudi_syllable_starts, nudi_to_unicode, syllable_starts, to_unicode, Conversion, Options,
    Unknown,
};
pub use encode::{from_unicode, unicode_to_nudi, Encoding, Unencodable};

use std::fmt;
use std::str::FromStr;

/// A legacy font glyph layout: which glyph each code draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Layout {
    /// Nudi 01 e to Nudi 05, Nudi Akshara and Baraha's BRH fonts for Kannada.
    Nudi,
    /// Akruti's Kannada fonts, such as AkrutiKndPadma.
    Akruti,
}

impl Layout {
    /// Every layout, in a fixed order.
    pub const ALL: [Layout; 2] = [Layout::Nudi, Layout::Akruti];

    /// The layout's name in lowercase, as [`FromStr`] reads it.
    ///
    /// ```
    /// use kannadi::legacy::Layout;
    ///
    /// assert_eq!(Layout::Akruti.name(), "akruti");
    /// assert_eq!("Akruti".parse(), Ok(Layout::Akruti));
    /// ```
    pub fn name(self) -> &'static str {
        match self {
            Layout::Nudi => "nudi",
            Layout::Akruti => "akruti",
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

impl fmt::Display for Layout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The error for a layout name [`Layout::from_str`] does not know.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownLayout(pub String);

impl fmt::Display for UnknownLayout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown layout {:?}", self.0)
    }
}

impl std::error::Error for UnknownLayout {}

impl FromStr for Layout {
    type Err = UnknownLayout;

    /// Reads a layout name, ignoring case. `baraha` is the Nudi layout.
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        if name.eq_ignore_ascii_case("baraha") {
            return Ok(Layout::Nudi);
        }
        Layout::ALL
            .into_iter()
            .find(|l| l.name().eq_ignore_ascii_case(name))
            .ok_or_else(|| UnknownLayout(name.to_string()))
    }
}

/// Decodes Windows-1252 bytes, the encoding legacy Kannada text files use.
/// The 5 bytes Windows-1252 leaves undefined map to the C1 control characters.
///
/// ```
/// assert_eq!(kannadi::legacy::decode_windows_1252(&[0x50, 0xC0, 0x86]), "PÀ†");
/// ```
pub fn decode_windows_1252(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| windows_1252(b)).collect()
}

/// The Windows-1252 byte for `c`, the inverse of [`decode_windows_1252`].
///
/// ```
/// use kannadi::legacy::windows_1252_byte;
///
/// assert_eq!(windows_1252_byte('À'), Some(0xC0));
/// assert_eq!(windows_1252_byte('†'), Some(0x86));
/// assert_eq!(windows_1252_byte('ಕ'), None);
/// ```
pub fn windows_1252_byte(c: char) -> Option<u8> {
    if let Some(i) = HIGH.iter().position(|&h| h == c) {
        return u8::try_from(0x80 + i).ok();
    }
    u8::try_from(u32::from(c))
        .ok()
        .filter(|b| !(0x80..=0x9F).contains(b))
}

const HIGH: [char; 32] = [
    '€', '\u{81}', '‚', 'ƒ', '„', '…', '†', '‡', 'ˆ', '‰', 'Š', '‹', 'Œ', '\u{8D}', 'Ž', '\u{8F}',
    '\u{90}', '‘', '’', '“', '”', '•', '–', '—', '˜', '™', 'š', '›', 'œ', '\u{9D}', 'ž', 'Ÿ',
];

fn windows_1252(b: u8) -> char {
    match b {
        0x80..=0x9F => HIGH[usize::from(b - 0x80)],
        _ => char::from(b),
    }
}

/// The consonants the mahaprana glyph turns ಪ ಬ ಡ ದ into.
const ASPIRATES: [(char, char); 4] = [('ಪ', 'ಫ'), ('ಬ', 'ಭ'), ('ಡ', 'ಢ'), ('ದ', 'ಧ')];

fn aspirated(c: char) -> Option<char> {
    ASPIRATES
        .iter()
        .find(|&&(plain, _)| plain == c)
        .map(|&(_, a)| a)
}

fn unaspirated(c: char) -> Option<char> {
    ASPIRATES
        .iter()
        .find(|&&(_, a)| a == c)
        .map(|&(plain, _)| plain)
}

/// Characters a legacy-font run carries as themselves: spaces and punctuation.
fn is_passthrough(c: char) -> bool {
    c.is_whitespace()
        || c.is_ascii_punctuation()
        || ('\u{2010}'..='\u{2027}').contains(&c)
        || c == '\u{A0}'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_1252_high_range() {
        assert_eq!(decode_windows_1252(&[0x86, 0x87, 0x84, 0x81]), "†‡„\u{81}");
    }

    #[test]
    fn windows_1252_roundtrips_every_byte() {
        for b in 0..=255u8 {
            let c = windows_1252(b);
            assert_eq!(windows_1252_byte(c), Some(b), "{b:02X}");
        }
        assert_eq!(windows_1252_byte('\u{0080}'), None);
    }

    #[test]
    fn layout_names_round_trip() {
        for layout in Layout::ALL {
            assert_eq!(layout.name().parse(), Ok(layout));
            assert_eq!(Layout::ALL[layout.index()], layout);
        }
        assert_eq!("BARAHA".parse(), Ok(Layout::Nudi));
        assert!("shree".parse::<Layout>().is_err());
    }

    #[test]
    fn aspirates_pair_both_ways() {
        for (plain, a) in ASPIRATES {
            assert_eq!(aspirated(plain), Some(a));
            assert_eq!(unaspirated(a), Some(plain));
        }
        assert_eq!(aspirated('ಕ'), None);
    }
}
