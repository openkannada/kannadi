//! Simple font encodings with a `Differences` list on a Windows-1252 base. lopdf
//! rejects a list that names a glyph it does not know, such as `/c129`, and reads
//! a list with no `BaseEncoding` on StandardEncoding, which misreads the codes
//! legacy fonts use. Here WinAnsi glyph names and `uniXXXX` names give their
//! characters, other names are read by lopdf where it can, and the rest keep their
//! code's Windows-1252 character.

use kannadi::legacy::decode_windows_1252;
use lopdf::{Dictionary, Document, Object};

use super::resolve;

/// Glyph names of WinAnsi codes 0x20 to 0xFF, in order. Codes with no glyph have
/// an empty name.
const WIN_ANSI: [&str; 224] = [
    "space",
    "exclam",
    "quotedbl",
    "numbersign",
    "dollar",
    "percent",
    "ampersand",
    "quotesingle",
    "parenleft",
    "parenright",
    "asterisk",
    "plus",
    "comma",
    "hyphen",
    "period",
    "slash",
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "colon",
    "semicolon",
    "less",
    "equal",
    "greater",
    "question",
    "at",
    "A",
    "B",
    "C",
    "D",
    "E",
    "F",
    "G",
    "H",
    "I",
    "J",
    "K",
    "L",
    "M",
    "N",
    "O",
    "P",
    "Q",
    "R",
    "S",
    "T",
    "U",
    "V",
    "W",
    "X",
    "Y",
    "Z",
    "bracketleft",
    "backslash",
    "bracketright",
    "asciicircum",
    "underscore",
    "grave",
    "a",
    "b",
    "c",
    "d",
    "e",
    "f",
    "g",
    "h",
    "i",
    "j",
    "k",
    "l",
    "m",
    "n",
    "o",
    "p",
    "q",
    "r",
    "s",
    "t",
    "u",
    "v",
    "w",
    "x",
    "y",
    "z",
    "braceleft",
    "bar",
    "braceright",
    "asciitilde",
    "",
    "Euro",
    "",
    "quotesinglbase",
    "florin",
    "quotedblbase",
    "ellipsis",
    "dagger",
    "daggerdbl",
    "circumflex",
    "perthousand",
    "Scaron",
    "guilsinglleft",
    "OE",
    "",
    "Zcaron",
    "",
    "",
    "quoteleft",
    "quoteright",
    "quotedblleft",
    "quotedblright",
    "bullet",
    "endash",
    "emdash",
    "tilde",
    "trademark",
    "scaron",
    "guilsinglright",
    "oe",
    "",
    "zcaron",
    "Ydieresis",
    "space",
    "exclamdown",
    "cent",
    "sterling",
    "currency",
    "yen",
    "brokenbar",
    "section",
    "dieresis",
    "copyright",
    "ordfeminine",
    "guillemotleft",
    "logicalnot",
    "hyphen",
    "registered",
    "macron",
    "degree",
    "plusminus",
    "twosuperior",
    "threesuperior",
    "acute",
    "mu",
    "paragraph",
    "periodcentered",
    "cedilla",
    "onesuperior",
    "ordmasculine",
    "guillemotright",
    "onequarter",
    "onehalf",
    "threequarters",
    "questiondown",
    "Agrave",
    "Aacute",
    "Acircumflex",
    "Atilde",
    "Adieresis",
    "Aring",
    "AE",
    "Ccedilla",
    "Egrave",
    "Eacute",
    "Ecircumflex",
    "Edieresis",
    "Igrave",
    "Iacute",
    "Icircumflex",
    "Idieresis",
    "Eth",
    "Ntilde",
    "Ograve",
    "Oacute",
    "Ocircumflex",
    "Otilde",
    "Odieresis",
    "multiply",
    "Oslash",
    "Ugrave",
    "Uacute",
    "Ucircumflex",
    "Udieresis",
    "Yacute",
    "Thorn",
    "germandbls",
    "agrave",
    "aacute",
    "acircumflex",
    "atilde",
    "adieresis",
    "aring",
    "ae",
    "ccedilla",
    "egrave",
    "eacute",
    "ecircumflex",
    "edieresis",
    "igrave",
    "iacute",
    "icircumflex",
    "idieresis",
    "eth",
    "ntilde",
    "ograve",
    "oacute",
    "ocircumflex",
    "otilde",
    "odieresis",
    "divide",
    "oslash",
    "ugrave",
    "uacute",
    "ucircumflex",
    "udieresis",
    "yacute",
    "thorn",
    "ydieresis",
];

/// The character for each code of a simple font with a `Differences` list and no
/// base encoding other than WinAnsi. `other_name` reads a code whose glyph name is
/// not a WinAnsi or `uniXXXX` name.
pub fn differences(
    doc: &Document,
    font: &Dictionary,
    other_name: impl Fn(u8) -> Option<char>,
) -> Option<Vec<char>> {
    let encoding = resolve(doc, font.get(b"Encoding").ok()?)?.as_dict().ok()?;
    let base = encoding.get(b"BaseEncoding").and_then(Object::as_name);
    if base.is_ok_and(|b| b != b"WinAnsiEncoding") {
        return None;
    }
    let list = resolve(doc, encoding.get(b"Differences").ok()?)?;
    let mut table: Vec<char> = (0..=255u8)
        .map(|b| {
            decode_windows_1252(&[b])
                .chars()
                .next()
                .unwrap_or('\u{FFFD}')
        })
        .collect();
    let mut code = 0usize;
    for item in list.as_array().ok()? {
        match item {
            Object::Integer(n) => code = usize::try_from(*n).unwrap_or(256),
            Object::Name(name) => {
                if let Some(slot) = table.get_mut(code) {
                    let byte = u8::try_from(code).unwrap_or_default();
                    if let Some(c) = glyph(name).or_else(|| other_name(byte)) {
                        *slot = c;
                    }
                }
                code += 1;
            }
            _ => {}
        }
    }
    Some(table)
}

/// Codes by string byte for a font named as legacy whose codes were renumbered
/// from below 32 and whose `Differences` list names its glyphs `G<n>`, as Acrobat
/// Distiller does when it converts the Akruti TrueType fonts: there the glyph
/// named `G112` is the font's code 112. Other bytes keep their own code. `None`
/// if the list starts at 32 or above, or no glyph has such a name.
pub fn glyph_codes(doc: &Document, font: &Dictionary) -> Option<Vec<char>> {
    let encoding = resolve(doc, font.get(b"Encoding").ok()?)?.as_dict().ok()?;
    let list = resolve(doc, encoding.get(b"Differences").ok()?)?;
    let mut table: Vec<char> = (0..=255u8)
        .map(|b| {
            decode_windows_1252(&[b])
                .chars()
                .next()
                .unwrap_or('\u{FFFD}')
        })
        .collect();
    let items = list.as_array().ok()?;
    let renumbered = matches!(items.first(), Some(Object::Integer(n)) if *n < 32);
    if !renumbered {
        return None;
    }
    let mut found = false;
    let mut code = 0usize;
    for item in items {
        match item {
            Object::Integer(n) => code = usize::try_from(*n).unwrap_or(256),
            Object::Name(name) => {
                if let (Some(slot), Some(n)) = (table.get_mut(code), code_name(name)) {
                    *slot = decode_windows_1252(&[n])
                        .chars()
                        .next()
                        .unwrap_or('\u{FFFD}');
                    found = true;
                }
                code += 1;
            }
            _ => {}
        }
    }
    found.then_some(table)
}

/// The code in a glyph name `G<n>`, with n from 0 to 255 written in decimal.
fn code_name(name: &[u8]) -> Option<u8> {
    let digits = name.strip_prefix(b"G")?;
    if digits.is_empty() || digits.len() > 3 || !digits.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(digits).ok()?.parse().ok()
}

fn glyph(name: &[u8]) -> Option<char> {
    let name = std::str::from_utf8(name).ok().filter(|n| !n.is_empty())?;
    if let Some(i) = WIN_ANSI.iter().position(|&n| n == name) {
        return decode_windows_1252(&[u8::try_from(0x20 + i).ok()?])
            .chars()
            .next();
    }
    let hex = name.strip_prefix("uni").filter(|h| h.len() == 4)?;
    char::from_u32(u32::from_str_radix(hex, 16).ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lopdf::dictionary;

    #[test]
    fn names_give_characters_and_unknown_names_keep_their_code() {
        let doc = Document::new();
        let font = dictionary! {
            "Encoding" => dictionary! { "Differences" => vec![
                Object::Integer(1),
                Object::Name(b"currency".to_vec()),
                Object::Name(b"Eth".to_vec()),
                Object::Name(b"c129".to_vec()),
                Object::Name(b"uni0C95".to_vec()),
                Object::Name(b"bullet".to_vec()),
                Object::Name(Vec::new()),
            ] },
        };
        let table = differences(&doc, &font, |_| None).unwrap();
        assert_eq!(table[1], '¤');
        assert_eq!(table[2], 'Ð');
        assert_eq!(table[3], '\u{3}');
        assert_eq!(table[4], 'ಕ');
        assert_eq!(table[5], '•');
        assert_eq!(table[6], '\u{6}');
        assert_eq!(table[0x41], 'A');
        let other = differences(&doc, &font, |b| (b == 3).then_some('ﬁ')).unwrap();
        assert_eq!(other[3], 'ﬁ');
    }

    #[test]
    fn other_base_encodings_are_left_to_lopdf() {
        let font = dictionary! {
            "Encoding" => dictionary! {
                "BaseEncoding" => Object::Name(b"MacRomanEncoding".to_vec()),
                "Differences" => vec![Object::Integer(65), Object::Name(b"B".to_vec())],
            },
        };
        assert!(differences(&Document::new(), &font, |_| None).is_none());
    }
}
