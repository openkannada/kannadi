//! A small HTML tokenizer. It finds tags, text, comments and the raw contents of
//! `<script>` and `<style>`, and keeps each token's source so that untouched parts
//! are written back unchanged.

use std::ops::Range;

#[derive(Debug, PartialEq, Eq)]
pub enum Token<'a> {
    Text(&'a str),
    Tag(Tag<'a>),
    /// The contents of a `<script>` or `<style>` element.
    Raw {
        element: &'static str,
        text: &'a str,
    },
    /// Comments, doctype and processing instructions.
    Other(&'a str),
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
    pub value: &'a str,
    /// Where `value` is in the tag's source; empty for an attribute with no value.
    pub span: Range<usize>,
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
        let end = if rest.starts_with("<!--") {
            rest.find("-->").map_or(html.len(), |e| at + e + 3)
        } else if rest.starts_with("<!") || rest.starts_with("<?") {
            rest.find('>').map_or(html.len(), |e| at + e + 1)
        } else if starts_tag(rest) {
            tag_end(html, at)
        } else {
            pos = at + 1;
            continue;
        };
        if text_start < at {
            tokens.push(Token::Text(&html[text_start..at]));
        }
        let source = &html[at..end];
        pos = end;
        if source.starts_with("<!") || source.starts_with("<?") {
            tokens.push(Token::Other(source));
        } else {
            let tag = parse_tag(source);
            let raw = match tag.name.as_str() {
                "script" if !tag.closing => Some("script"),
                "style" if !tag.closing => Some("style"),
                _ => None,
            };
            tokens.push(Token::Tag(tag));
            if let Some(element) = raw {
                let close = find_ignore_case(&html[pos..], &format!("</{element}"))
                    .map_or(html.len(), |e| pos + e);
                if pos < close {
                    tokens.push(Token::Raw {
                        element,
                        text: &html[pos..close],
                    });
                }
                pos = close;
            }
        }
        text_start = pos;
    }
    if text_start < html.len() {
        tokens.push(Token::Text(&html[text_start..]));
    }
    tokens
}

fn starts_tag(rest: &str) -> bool {
    let after = rest.strip_prefix("</").unwrap_or(&rest[1..]);
    after.starts_with(|c: char| c.is_ascii_alphabetic())
}

/// The end of the tag starting at `at`: after the first `>` outside quotes.
fn tag_end(html: &str, at: usize) -> usize {
    let mut quote = None;
    for (i, c) in html[at..].char_indices() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), c) if c == q => quote = None,
            (None, '>') => return at + i + 1,
            _ => {}
        }
    }
    html.len()
}

fn parse_tag(source: &str) -> Tag<'_> {
    let closing = source.starts_with("</");
    let body = &source[if closing { 2 } else { 1 }..];
    let name_len = body
        .find(|c: char| c.is_whitespace() || c == '>' || c == '/')
        .unwrap_or(body.len());
    let name = body[..name_len].to_ascii_lowercase();
    let offset = source.len() - body.len() + name_len;
    Tag {
        source,
        name,
        closing,
        attrs: parse_attrs(source, offset),
    }
}

fn parse_attrs(source: &str, mut i: usize) -> Vec<Attr<'_>> {
    let bytes = source.as_bytes();
    let mut attrs = Vec::new();
    let skip_space = |mut i: usize| {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'/') {
            i += 1;
        }
        i
    };
    loop {
        i = skip_space(i);
        if i >= bytes.len() || bytes[i] == b'>' {
            return attrs;
        }
        let start = i;
        while i < bytes.len()
            && !matches!(bytes[i], b'=' | b'>' | b'/')
            && !bytes[i].is_ascii_whitespace()
        {
            i += 1;
        }
        let name = source[start..i].to_ascii_lowercase();
        let mut j = i;
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        let mut span = i..i;
        if j < bytes.len() && bytes[j] == b'=' {
            j += 1;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j < bytes.len() && (bytes[j] == b'"' || bytes[j] == b'\'') {
                let q = bytes[j];
                let end = source[j + 1..]
                    .find(char::from(q))
                    .map_or(bytes.len(), |e| j + 1 + e);
                span = j + 1..end;
                i = (end + 1).min(bytes.len());
            } else {
                let end = source[j..]
                    .find(|c: char| c.is_whitespace() || c == '>')
                    .map_or(bytes.len(), |e| j + e);
                span = j..end;
                i = end;
            }
        }
        if name.is_empty() {
            i += 1;
            continue;
        }
        attrs.push(Attr {
            name,
            value: &source[span.clone()],
            span,
        });
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

    #[test]
    fn tokens_cover_the_source() {
        let html = "<!DOCTYPE html><p class=x STYLE='font-family:\"BRH Kannada\"'>a < b<!-- c --></p><style>p{}</style><br/>";
        let tokens = tokenize(html);
        let joined: String = tokens
            .iter()
            .map(|t| match t {
                Token::Text(s) | Token::Other(s) => *s,
                Token::Tag(t) => t.source,
                Token::Raw { text, .. } => text,
            })
            .collect();
        assert_eq!(joined, html);
        let Token::Tag(p) = &tokens[1] else { panic!() };
        assert_eq!(p.name, "p");
        assert_eq!(p.attr("class").unwrap().value, "x");
        assert_eq!(
            p.attr("style").unwrap().value,
            "font-family:\"BRH Kannada\""
        );
        assert_eq!(tokens[2], Token::Text("a < b"));
        assert!(matches!(
            tokens[6],
            Token::Raw {
                element: "style",
                text: "p{}"
            }
        ));
    }

    #[test]
    fn attribute_spans_point_into_the_tag() {
        let tag = parse_tag("<font face = \"BRH Kannada\" size=3>");
        let face = tag.attr("face").unwrap();
        assert_eq!(&tag.source[face.span.clone()], "BRH Kannada");
        assert_eq!(tag.attr("size").unwrap().value, "3");
        assert!(parse_tag("</P>").closing);
    }
}
