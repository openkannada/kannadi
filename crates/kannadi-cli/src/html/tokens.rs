//! A small HTML tokenizer. It finds tags, text, comments and the raw contents of
//! `<script>`, `<style>`, `<title>` and `<textarea>`, and keeps each token's source
//! so untouched parts are written back unchanged. Tokens always cover the input.

use std::ops::Range;

/// Elements whose content is text and never markup.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RawKind {
    Script,
    Style,
    /// `<title>` and `<textarea>`: text that cannot take a font.
    Plain,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Token<'a> {
    Text(&'a str),
    Tag(Tag<'a>),
    Raw {
        kind: RawKind,
        text: &'a str,
    },
    /// Comments, doctype, processing instructions and bogus end tags.
    Other(&'a str),
}

impl<'a> Token<'a> {
    pub fn source(&self) -> &'a str {
        match self {
            Token::Text(s) | Token::Other(s) | Token::Raw { text: s, .. } => s,
            Token::Tag(t) => t.source,
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Tag<'a> {
    pub source: &'a str,
    /// Lowercase.
    pub name: String,
    pub closing: bool,
    pub attrs: Vec<Attr<'a>>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Attr<'a> {
    /// Lowercase.
    pub name: String,
    /// As written, with character references not decoded.
    pub value: &'a str,
    /// Where `value` is in the tag's source. For an attribute with no value, an
    /// empty range just after the name.
    pub span: Range<usize>,
    pub quote: Option<char>,
}

impl Tag<'_> {
    pub fn attr(&self, name: &str) -> Option<&Attr<'_>> {
        self.attrs.iter().find(|a| a.name == name)
    }
}

pub fn tokenize(html: &str) -> Vec<Token<'_>> {
    let mut tokens = Vec::new();
    let mut pos = 0;
    let mut text_start = 0;
    while let Some(found) = html[pos..].find('<') {
        let at = pos + found;
        let rest = &html[at..];
        let after = rest[1..].chars().next();
        let (token, end) = if rest.starts_with("<!--") {
            let end = rest
                .strip_prefix("<!--")
                .and_then(|r| r.find("-->"))
                .map_or(html.len(), |e| at + 4 + e + 3);
            (Token::Other(&html[at..end]), end)
        } else if rest.starts_with("<!") || rest.starts_with("<?") || rest.starts_with("</") {
            if rest.starts_with("</") && rest[2..].starts_with(|c: char| c.is_ascii_alphabetic()) {
                let tag = parse_tag(html, at);
                let end = at + tag.source.len();
                (Token::Tag(tag), end)
            } else {
                let end = rest.find('>').map_or(html.len(), |e| at + e + 1);
                (Token::Other(&html[at..end]), end)
            }
        } else if after.is_some_and(|c| c.is_ascii_alphabetic()) {
            let tag = parse_tag(html, at);
            let end = at + tag.source.len();
            (Token::Tag(tag), end)
        } else {
            pos = at + 1;
            continue;
        };
        if text_start < at {
            tokens.push(Token::Text(&html[text_start..at]));
        }
        let raw = match &token {
            Token::Tag(t) if !t.closing => match t.name.as_str() {
                "script" => Some(RawKind::Script),
                "style" => Some(RawKind::Style),
                "title" | "textarea" => Some(RawKind::Plain),
                _ => None,
            },
            _ => None,
        };
        let raw_close = match &token {
            Token::Tag(t) => format!("</{}", t.name),
            _ => String::new(),
        };
        tokens.push(token);
        pos = end;
        if let Some(kind) = raw {
            let close = find_ignore_case(&html[pos..], &raw_close).map_or(html.len(), |e| pos + e);
            if pos < close {
                tokens.push(Token::Raw {
                    kind,
                    text: &html[pos..close],
                });
            }
            pos = close;
        }
        text_start = pos;
    }
    if text_start < html.len() {
        tokens.push(Token::Text(&html[text_start..]));
    }
    tokens
}

/// Parses the tag that starts at `at`. Only ASCII bytes end a name or value, so
/// every slice falls on a character boundary.
fn parse_tag(html: &str, at: usize) -> Tag<'_> {
    let bytes = html.as_bytes();
    let closing = html[at..].starts_with("</");
    let mut i = at + if closing { 2 } else { 1 };
    let is_space = |b: u8| b.is_ascii_whitespace();
    let name_start = i;
    while i < bytes.len() && !is_space(bytes[i]) && bytes[i] != b'>' && bytes[i] != b'/' {
        i += 1;
    }
    let name = html[name_start..i].to_ascii_lowercase();
    let mut attrs = Vec::new();
    loop {
        while i < bytes.len() && (is_space(bytes[i]) || bytes[i] == b'/') {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        if bytes[i] == b'>' {
            i += 1;
            break;
        }
        let start = i;
        // A tag may start an attribute name with '=', which then belongs to the name.
        i += 1;
        while i < bytes.len() && !is_space(bytes[i]) && !matches!(bytes[i], b'=' | b'>' | b'/') {
            i += 1;
        }
        let attr_name = html[start..i].to_ascii_lowercase();
        let mut j = i;
        while j < bytes.len() && is_space(bytes[j]) {
            j += 1;
        }
        let (span, quote) = if j < bytes.len() && bytes[j] == b'=' {
            j += 1;
            while j < bytes.len() && is_space(bytes[j]) {
                j += 1;
            }
            match bytes.get(j) {
                Some(&q @ (b'"' | b'\'')) => {
                    let end = html[j + 1..]
                        .find(char::from(q))
                        .map_or(html.len(), |e| j + 1 + e);
                    i = (end + 1).min(html.len());
                    (j + 1..end, Some(char::from(q)))
                }
                _ => {
                    let end = html[j..]
                        .find(|c: char| c.is_ascii_whitespace() || c == '>')
                        .map_or(html.len(), |e| j + e);
                    i = end;
                    (j..end, None)
                }
            }
        } else {
            (i..i, None)
        };
        attrs.push(Attr {
            name: attr_name,
            value: &html[span.clone()],
            span: span.start - at..span.end - at,
            quote,
        });
    }
    Tag {
        source: &html[at..i],
        name,
        closing,
        attrs,
    }
}

fn find_ignore_case(haystack: &str, needle: &str) -> Option<usize> {
    haystack
        .as_bytes()
        .windows(needle.len())
        .position(|w| w.eq_ignore_ascii_case(needle.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn joined(html: &str) -> String {
        tokenize(html).iter().map(Token::source).collect()
    }

    #[test]
    fn tokens_cover_the_source() {
        let html =
            "<!DOCTYPE html><p class=x STYLE='font-family:\"BRH Kannada\"'>a < b<!-- c --></p>\
                    <style>p{}</style><br/><title>t</title>";
        let tokens = tokenize(html);
        assert_eq!(joined(html), html);
        let Token::Tag(p) = &tokens[1] else { panic!() };
        assert_eq!(p.name, "p");
        assert_eq!(p.attr("class").unwrap().value, "x");
        assert_eq!(p.attr("style").unwrap().quote, Some('\''));
        assert_eq!(tokens[2], Token::Text("a < b"));
        assert!(matches!(
            tokens[6],
            Token::Raw {
                kind: RawKind::Style,
                text: "p{}"
            }
        ));
        assert!(matches!(
            tokens[10],
            Token::Raw {
                kind: RawKind::Plain,
                text: "t"
            }
        ));
    }

    #[test]
    fn attribute_spans_point_into_the_tag() {
        let tag = parse_tag("<font face = \"BRH Kannada\" size=3 hidden>", 0);
        let face = tag.attr("face").unwrap();
        assert_eq!(&tag.source[face.span.clone()], "BRH Kannada");
        assert_eq!(tag.attr("size").unwrap().value, "3");
        assert_eq!(tag.attr("hidden").unwrap().span, 40..40);
        assert!(parse_tag("</P>", 0).closing);
    }

    #[test]
    fn quotes_only_open_after_an_equals_sign() {
        let html = "<p title=O'Brien>x</p><p style=\"font-family:BRH\">y</p>";
        let tokens = tokenize(html);
        assert_eq!(tokens.len(), 6, "{tokens:?}");
        assert_eq!(tokens[1], Token::Text("x"));
    }

    #[test]
    fn odd_input_never_breaks_the_tokenizer() {
        for html in [
            "<p =\"a\"ಕ>x</p>",
            "<p/=\"tP\"ಕ>",
            "</ p>ಕ",
            "<p a='ಕ",
            "<!--ಕ",
            "<a\u{0CCD}>",
            "<",
        ] {
            assert_eq!(joined(html), html);
        }
        assert!(matches!(tokenize("</ p>")[0], Token::Other("</ p>")));
    }

    proptest::proptest! {
        #[test]
        fn tokens_cover_any_input(s in "[<>/=\"' a-zA-Z!?\\-ಕ಼್ಾ&;#\n]{0,48}") {
            proptest::prop_assert_eq!(joined(&s), s);
        }

        #[test]
        fn tokens_cover_any_string(s in proptest::prelude::any::<String>()) {
            proptest::prop_assert_eq!(joined(&s), s);
        }
    }
}
