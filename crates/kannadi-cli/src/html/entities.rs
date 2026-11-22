//! Character references in HTML text: legacy glyph codes are often written as
//! `&Agrave;` or `&#192;`.

use kannadi::legacy::windows_1252_char;

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

/// Names for the other characters Windows-1252 has, the 5 XML ones, and the
/// spaces and joiners Kannada text uses.
const OTHER: [(&str, char); 39] = [
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
    ("zwnj", '\u{200C}'),
    ("zwj", '\u{200D}'),
    ("ensp", '\u{2002}'),
    ("emsp", '\u{2003}'),
    ("thinsp", '\u{2009}'),
    ("lrm", '\u{200E}'),
    ("rlm", '\u{200F}'),
];

/// Text with character references decoded, mapped back to its source.
pub struct Decoded {
    pub text: String,
    /// For each character of `text`: its byte offset there and in the source.
    starts: Vec<(usize, usize)>,
    source_len: usize,
}

impl Decoded {
    /// The source offset where the character at byte `at` of `text` starts.
    pub fn source_offset(&self, at: usize) -> usize {
        match self.starts.binary_search_by_key(&at, |&(t, _)| t) {
            Ok(i) => self.starts[i].1,
            Err(_) => self.source_len,
        }
    }
}

/// `text` with references replaced. Unknown references stay as written. Numeric
/// references to 128 to 159 mean the Windows-1252 characters, as browsers read them.
pub fn decode(text: &str) -> Decoded {
    let mut out = Decoded {
        text: String::with_capacity(text.len()),
        starts: Vec::with_capacity(text.len()),
        source_len: text.len(),
    };
    let mut i = 0;
    while i < text.len() {
        let rest = &text[i..];
        let reference = rest
            .strip_prefix('&')
            .and_then(|r| {
                let window = &r.as_bytes()[..r.len().min(32)];
                window.iter().position(|&b| b == b';').map(|e| &r[..e])
            })
            .and_then(|name| lookup(name).map(|c| (c, name.len() + 2)));
        let (c, len) = reference.unwrap_or_else(|| {
            let c = rest.chars().next().unwrap_or_default();
            (c, c.len_utf8())
        });
        out.starts.push((out.text.len(), i));
        out.text.push(c);
        i += len;
    }
    out
}

fn lookup(name: &str) -> Option<char> {
    if let Some(number) = name.strip_prefix('#') {
        let code = match number.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => number.parse().ok()?,
        };
        return match u8::try_from(code) {
            Ok(b @ 0x80..=0x9F) => Some(windows_1252_char(b)),
            _ => char::from_u32(code),
        };
    }
    if let Some(i) = LATIN_1.iter().position(|&n| n == name) {
        return char::from_u32(0xA0 + u32::try_from(i).ok()?);
    }
    OTHER.iter().find(|(n, _)| *n == name).map(|&(_, c)| c)
}

/// `text` escaped for element content, or for an attribute value in `quote` marks.
pub fn escape(text: &str, quote: Option<char>) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' if quote.is_none() => out.push_str("&lt;"),
            '>' if quote.is_none() => out.push_str("&gt;"),
            '"' if quote == Some('"') => out.push_str("&quot;"),
            '\'' if quote == Some('\'') => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn references_decode_and_map_back() {
        let source = "P&Agrave;&#163;&#xC0;&#134; &amp;&nbsp;&zwnj;";
        let d = decode(source);
        assert_eq!(d.text, "PÀ£À† &\u{A0}\u{200C}");
        assert_eq!(d.source_offset(1), 1);
        assert_eq!(d.source_offset(3), 9);
        assert_eq!(d.source_offset(d.text.len()), source.len());
        assert_eq!(decode("a & b &unknown; &#;").text, "a & b &unknown; &#;");
        assert_eq!(decode("&#99999999;&").text, "&#99999999;&");
        let near_multibyte = format!("&#3221;{}", "ಕ".repeat(10));
        assert_eq!(decode(&near_multibyte).text, "ಕ".repeat(11));
    }

    #[test]
    fn escapes_suit_the_context() {
        assert_eq!(escape("a<b>&c\u{A0}", None), "a&lt;b&gt;&amp;c\u{A0}");
        assert_eq!(escape("'A' \"B\"", Some('"')), "'A' &quot;B&quot;");
        assert_eq!(escape("'A'", Some('\'')), "&#39;A&#39;");
    }
}
