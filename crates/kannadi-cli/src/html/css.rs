//! The parts of CSS that decide fonts: `font-family` declarations, and rules whose
//! selector is a tag, a class, or a tag with a class.

use std::ops::Range;

#[derive(Debug, PartialEq, Eq)]
enum Selector {
    /// Lowercase.
    Tag(String),
    /// Class names are case-sensitive.
    Class(String),
    TagClass(String, String),
}

impl Selector {
    fn parse(text: &str) -> Option<Self> {
        let simple = |s: &str| {
            !s.is_empty()
                && s.chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        };
        match text.split_once('.') {
            None if simple(text) => Some(Selector::Tag(text.to_ascii_lowercase())),
            Some(("", class)) if simple(class) => Some(Selector::Class(class.to_string())),
            Some((tag, class)) if simple(tag) && simple(class) => Some(Selector::TagClass(
                tag.to_ascii_lowercase(),
                class.to_string(),
            )),
            _ => None,
        }
    }

    fn specificity(&self) -> u8 {
        match self {
            Selector::Tag(_) => 1,
            Selector::Class(_) => 10,
            Selector::TagClass(..) => 11,
        }
    }

    fn matches(&self, tag: &str, classes: &[&str]) -> bool {
        match self {
            Selector::Tag(t) => t == tag,
            Selector::Class(c) => classes.contains(&c.as_str()),
            Selector::TagClass(t, c) => t == tag && classes.contains(&c.as_str()),
        }
    }
}

/// `font-family` lists from `<style>` rules with simple selectors, in source order.
#[derive(Debug, Default)]
pub struct Rules {
    rules: Vec<(Selector, String)>,
}

impl Rules {
    /// Adds the rules in 1 stylesheet.
    pub fn add(&mut self, css: &str) {
        for (selectors, body) in blocks(css) {
            let Some(family) = font_family(&css[body]) else {
                continue;
            };
            let selectors = css[selectors].trim().trim_start_matches("<!--");
            for selector in selectors
                .split(',')
                .filter_map(|s| Selector::parse(s.trim()))
            {
                self.rules.push((selector, family.clone()));
            }
        }
    }

    /// The `font-family` list for an element: from the most specific matching rule,
    /// and among equally specific ones, the last.
    pub fn family(&self, tag: &str, classes: &str) -> Option<&str> {
        let classes: Vec<&str> = classes.split_whitespace().collect();
        self.rules
            .iter()
            .enumerate()
            .filter(|(_, (s, _))| s.matches(tag, &classes))
            .max_by_key(|(order, (s, _))| (s.specificity(), *order))
            .map(|(_, (_, family))| family.as_str())
    }
}

/// What a `font-family` value asks for.
#[derive(Debug, PartialEq, Eq)]
pub enum Family<'a> {
    Named(&'a str),
    /// `inherit`, `unset` and `revert`: the parent's font, since `font-family` is
    /// inherited and a page's styles are the only author styles read here.
    Inherit,
    /// `initial`: the browser's default font.
    Initial,
}

impl<'a> Family<'a> {
    pub fn of(value: &'a str) -> Self {
        let bare = value.trim().trim_end_matches("!important").trim();
        if ["inherit", "unset"]
            .iter()
            .any(|k| bare.eq_ignore_ascii_case(k))
        {
            Family::Inherit
        } else if ["initial", "revert"]
            .iter()
            .any(|k| bare.eq_ignore_ascii_case(k))
        {
            Family::Initial
        } else {
            Family::Named(value)
        }
    }
}

/// The value of the last `font-family` declaration in a declaration block, with
/// comments removed. An empty value counts as no declaration.
pub fn font_family(block: &str) -> Option<String> {
    let value = declarations(block)
        .filter(|(name, _)| {
            without_comments(&block[name.clone()])
                .trim()
                .eq_ignore_ascii_case("font-family")
        })
        .last()
        .map(|(_, value)| without_comments(&block[value]).trim().to_string())?;
    (!value.is_empty()).then_some(value)
}

fn without_comments(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        rest = rest[start + 2..]
            .find("*/")
            .map_or("", |e| &rest[start + 2 + e + 2..]);
    }
    out.push_str(rest);
    out
}

/// `css` with every `font-family` value passed through `replace`. `css` is a
/// stylesheet when `stylesheet` is set, or else 1 declaration block.
pub fn rewrite_font_families(
    css: &str,
    stylesheet: bool,
    replace: impl Fn(&str) -> String,
) -> String {
    let bodies: Vec<Range<usize>> = if stylesheet {
        blocks(css).into_iter().map(|(_, body)| body).collect()
    } else {
        std::iter::once(0..css.len()).collect()
    };
    let mut edits = Vec::new();
    for body in bodies {
        for (name, value) in declarations(&css[body.clone()]) {
            let name = body.start + name.start..body.start + name.end;
            if without_comments(&css[name])
                .trim()
                .eq_ignore_ascii_case("font-family")
            {
                let value = body.start + value.start..body.start + value.end;
                edits.push((value.clone(), replace(&css[value])));
            }
        }
    }
    crate::edits::apply(css, edits)
}

/// Selector and body ranges of each rule in a stylesheet, with comments skipped.
fn blocks(css: &str) -> Vec<(Range<usize>, Range<usize>)> {
    let mut blocks = Vec::new();
    let mut start = 0;
    let mut open = None;
    let mut quote = None;
    let mut chars = css.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, '/') if chars.peek().is_some_and(|&(_, n)| n == '*') => {
                let end = css[i + 2..].find("*/").map_or(css.len(), |e| i + 2 + e + 2);
                while chars.peek().is_some_and(|&(j, _)| j < end) {
                    chars.next();
                }
                if open.is_none() {
                    start = end;
                }
            }
            (None, '{') if open.is_none() => open = Some(i),
            (None, '}') => {
                if let Some(o) = open.take() {
                    blocks.push((start..o, o + 1..i));
                }
                start = i + 1;
            }
            _ => {}
        }
    }
    blocks
}

/// Name and value ranges of each declaration in a block. Values end at `;` outside
/// quotes or at the end of the block.
fn declarations(block: &str) -> impl Iterator<Item = (Range<usize>, Range<usize>)> + '_ {
    let mut start = 0;
    let mut quote = None;
    let mut ends = Vec::new();
    for (i, c) in block.char_indices() {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), _) => {}
            (None, '"' | '\'') => quote = Some(c),
            (None, ';') => {
                ends.push(start..i);
                start = i + 1;
            }
            _ => {}
        }
    }
    ends.push(start..block.len());
    ends.into_iter().filter_map(move |d| {
        let colon = block[d.clone()].find(':')?;
        Some((d.start..d.start + colon, d.start + colon + 1..d.end))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_selectors_give_families() {
        let mut rules = Rules::default();
        rules.add(
            "<!--table {}\n.font12 {color:blue; font-family:\"BRH Sirigannada\";}\n\
             td.x, P {font-family: Arial} /* td {font-family: Nudi 01 e} */ div p {font-family: Y}\n\
             .a{font-family:Arial} .b{font-family:BRH Kannada} .Up{font-family:Z}",
        );
        assert_eq!(rules.family("font", "font12"), Some("\"BRH Sirigannada\""));
        assert_eq!(rules.family("td", "x"), Some("Arial"));
        assert_eq!(rules.family("p", ""), Some("Arial"));
        assert_eq!(rules.family("td", ""), None);
        assert_eq!(rules.family("div", ""), None);
        assert_eq!(rules.family("p", "b a"), Some("BRH Kannada"));
        assert_eq!(rules.family("p", "up"), Some("Arial"));
        assert_eq!(rules.family("p", "Up"), Some("Z"));
    }

    #[test]
    fn declarations_and_rewrites() {
        let family = |b: &str| font_family(b);
        let quoted = family("color:red; FONT-FAMILY: 'BRH Kannada'");
        assert_eq!(quoted.as_deref(), Some("'BRH Kannada'"));
        assert_eq!(family("font-family:\"A;B\"").as_deref(), Some("\"A;B\""));
        assert_eq!(family("/* x */font-family:/* y */ B").as_deref(), Some("B"));
        assert_eq!(family("font-size:10pt"), None);
        assert_eq!(family("font-family:"), None);
        assert_eq!(Family::of("inherit"), Family::Inherit);
        assert_eq!(Family::of(" Initial "), Family::Initial);
        assert_eq!(Family::of("Arial"), Family::Named("Arial"));
        let css = "p{font-family:\"A;B\";mso-font-family:x} q{font-family: C}";
        let out = rewrite_font_families(css, true, |v| format!("[{v}]"));
        assert_eq!(
            out,
            "p{font-family:[\"A;B\"];mso-font-family:x} q{font-family:[ C]}"
        );
        let style = rewrite_font_families("color:red;font-family:X", false, |v| format!("[{v}]"));
        assert_eq!(style, "color:red;font-family:[X]");
    }
}
