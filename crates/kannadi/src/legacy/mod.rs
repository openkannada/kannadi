//! Nudi and Baraha ANSI text to and from Unicode.
//!
//! Before Unicode, Kannada was typed in fonts that draw Kannada shapes at Latin-1
//! code points. Nudi (up to version 5) and Baraha's ANSI fonts (BRH Kannada and
//! others) share 1 glyph layout. Text in those fonts reads as `PÀ£ÀßqÀ` until it
//! is converted.
//!
//! Glyphs are stored in drawing order: a subscript consonant comes after its
//! syllable's vowel sign, the arkavattu after the cluster, and some vowel signs
//! are split into 2 or 3 glyphs. [`nudi_to_unicode`] rebuilds each syllable in
//! Unicode order. It reports every glyph it cannot place and every syllable that
//! is not valid; spacing glyphs, which carry no text, are the only glyphs dropped.
//! [`unicode_to_nudi`] goes the other way, for documents that must still be
//! printed in a legacy font, and reports every character it cannot encode.

mod decode;
mod encode;
mod table;

pub use decode::{nudi_syllable_starts, nudi_to_unicode, Conversion, Options, Unknown};
pub use encode::{unicode_to_nudi, Encoding, Unencodable};

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
    fn aspirates_pair_both_ways() {
        for (plain, a) in ASPIRATES {
            assert_eq!(aspirated(plain), Some(a));
            assert_eq!(unaspirated(a), Some(plain));
        }
        assert_eq!(aspirated('ಕ'), None);
    }
}
