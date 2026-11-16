//! Simple font encodings with a `Differences` list. lopdf drops the whole list
//! when it names a glyph it does not know, such as `/c129`, so the list is read
//! here: WinAnsi glyph names give their characters, and other names keep their
//! code's Windows-1252 character.

use kannadi::legacy::decode_windows_1252;
use lopdf::{Dictionary, Document, Object};

/// Glyph names of WinAnsi codes 0x20 to 0xFF, in order.
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
    "bullet",
    "Euro",
    "bullet",
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
    "bullet",
    "Zcaron",
    "bullet",
    "bullet",
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
    "bullet",
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

/// The character for each code of a simple font with a `Differences` list.
pub fn differences(doc: &Document, font: &Dictionary) -> Option<Vec<char>> {
    let resolve = |o: &Object| match o {
        Object::Reference(id) => doc.get_object(*id).ok().cloned(),
        other => Some(other.clone()),
    };
    let encoding = resolve(font.get(b"Encoding").ok()?)?;
    let list = resolve(encoding.as_dict().ok()?.get(b"Differences").ok()?)?;
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
                if let (Some(slot), Some(c)) = (table.get_mut(code), glyph(name)) {
                    *slot = c;
                }
                code += 1;
            }
            _ => {}
        }
    }
    Some(table)
}

fn glyph(name: &[u8]) -> Option<char> {
    let name = std::str::from_utf8(name).ok()?;
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
            ] },
        };
        let table = differences(&doc, &font).unwrap();
        assert_eq!(table[1], '¤');
        assert_eq!(table[2], 'Ð');
        assert_eq!(table[3], '\u{3}');
        assert_eq!(table[4], 'ಕ');
        assert_eq!(table[0x41], 'A');
    }
}
