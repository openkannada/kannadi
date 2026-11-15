//! Character references in HTML text: legacy glyph codes are often written as
//! `&Agrave;` or `&#192;`.

use kannadi::legacy::decode_windows_1252;

/// HTML 4 names for U+00A0 to U+00FF, in order.
const LATIN_1: [&str; 96] = [
    "nbsp", "iexcl", "cent", "pound", "curren", "yen", "brvbar", "sect", "uml", "copy", "ordf",
    "laquo", "not", "shy", "reg", "macr", "deg", "plusmn", "sup2", "sup3", "acute", "micro",
    "para", "middot", "cedil", "sup1", "ordm", "raquo", "frac14", "frac12", "frac34", "iquest",
    "Agrave", "Aacute", "Acirc", "Atilde", "Auml", "Aring", "AElig", "Ccedil", "Egrave", "Eacute",
    "Ecirc", "Euml", "Igrave", "Iacute", "Icirc", "Iuml", "ETH", "Ntilde", "Ograve", "Oacute",
    "Ocirc", "Otilde", "Ouml", "times", "Oslash", "Ugrave", "Uacute", "Ucirc", "Uuml", "Yacute",
    "THORN", "szlig", "agrave", "aacute", "acirc", "atilde", "auml", "aring", "aelig", "ccedil",
    "egrave", "eacute", "ecirc", "euml", "igrave", "iacute", "icirc", "iuml", "eth", "ntilde",
    "ograve", "oacute", "ocirc", "otilde", "ouml", "divide", "oslash", "ugrave", "uacute", "ucirc",
    "uuml", "yacute", "thorn", "yuml",
];

/// Names for the other characters Windows-1252 has, and the 5 XML ones.
const OTHER: [(&str, char); 32] = [
    ("amp", '&'),
    ("lt", '<'),
    ("gt", '>'),
    ("quot", '"'),
    ("apos", '\''),
    ("euro", '€'),
    ("sbquo", '‚'),
    ("fnof", 'ƒ'),
    ("bdquo", '„'),
    ("hellip", '…'),
    ("dagger", '†'),
    ("Dagger", '‡'),
    ("circ", 'ˆ'),
    ("permil", '‰'),
    ("Scaron", 'Š'),
    ("lsaquo", '‹'),
    ("OElig", 'Œ'),
    ("Zcaron", 'Ž'),
    ("lsquo", '‘'),
    ("rsquo", '’'),
    ("ldquo", '“'),
    ("rdquo", '”'),
    ("bull", '•'),
    ("ndash", '–'),
    ("mdash", '—'),
    ("tilde", '˜'),
    ("trade", '™'),
    ("scaron", 'š'),
    ("rsaquo", '›'),
    ("oelig", 'œ'),
    ("zcaron", 'ž'),
    ("Yuml", 'Ÿ'),
];

/// `text` with character references replaced. Unknown references stay as written.
/// Numeric references to 128 to 159 mean the Windows-1252 characters, as browsers
/// read them.
pub fn decode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let reference = rest[1..]
            .find(';')
            .filter(|&e| e <= 10)
            .map(|e| &rest[1..1 + e]);
        match reference.and_then(lookup) {
            Some(c) => {
                out.push(c);
                rest = &rest[reference.map_or(0, str::len) + 2..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn lookup(name: &str) -> Option<char> {
    if let Some(number) = name.strip_prefix('#') {
        let code = match number.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => number.parse().ok()?,
        };
        return match u8::try_from(code) {
            Ok(b @ 0x80..=0x9F) => decode_windows_1252(&[b]).chars().next(),
            _ => char::from_u32(code),
        };
    }
    if let Some(i) = LATIN_1.iter().position(|&n| n == name) {
        return char::from_u32(0xA0 + u32::try_from(i).ok()?);
    }
    OTHER.iter().find(|(n, _)| *n == name).map(|&(_, c)| c)
}

/// `text` with `&`, `<` and `>` escaped.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references_decode() {
        assert_eq!(
            decode("P&Agrave;&#163;&#xC0;&#134; &amp;&nbsp;"),
            "PÀ£À† &\u{A0}"
        );
        assert_eq!(decode("a & b &unknown; &#;"), "a & b &unknown; &#;");
        assert_eq!(decode("&yuml;&dagger;"), "ÿ†");
    }

    #[test]
    fn escape_is_minimal() {
        assert_eq!(escape("a<b>&c\u{A0}"), "a&lt;b&gt;&amp;c\u{A0}");
    }
}
