//! The parts of CSS that decide fonts: `font-family` in declarations, and rules
//! whose selector is a tag, a class, or a tag with a class.

use std::collections::HashMap;

/// `font-family` lists by simple selector: `p`, `.x` or `p.x`, in lowercase.
#[derive(Debug, Default)]
pub struct Rules {
    families: HashMap<String, String>,
}

impl Rules {
    /// Adds the rules in a `<style>` element. Later rules win over earlier ones.
    pub fn add(&mut self, css: &str) {
        let css = strip_comments(css);
        for block in css.split('}') {
            let Some((selectors, declarations)) = block.split_once('{') else {
                continue;
            };
            let Some(family) = font_family(declarations) else {
                continue;
            };
            for selector in selectors.split(',') {
                let selector = selector.trim().trim_start_matches("<!--").trim();
                let simple = !selector.is_empty()
                    && selector
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
                if simple {
                    self.families
                        .insert(selector.to_ascii_lowercase(), family.to_string());
                }
            }
        }
    }

    /// The `font-family` list for an element, from the most specific matching rule.
    pub fn family(&self, tag: &str, classes: &str) -> Option<&str> {
        let classes: Vec<String> = classes
            .split_whitespace()
            .map(str::to_ascii_lowercase)
            .collect();
        let with_tag = classes.iter().map(|c| format!("{tag}.{c}"));
        let alone = classes.iter().map(|c| format!(".{c}"));
        with_tag
            .chain(alone)
            .chain(std::iter::once(tag.to_string()))
            .find_map(|s| self.families.get(&s))
            .map(String::as_str)
    }
}

/// The value of the last `font-family` declaration in `declarations`.
pub fn font_family(declarations: &str) -> Option<&str> {
    declarations
        .split(';')
        .rev()
        .filter_map(|d| d.split_once(':'))
        .filter(|(name, _)| name.trim().eq_ignore_ascii_case("font-family"))
        .map(|(_, value)| value.trim())
        .next()
}

/// `css` with every `font-family` value passed through `replace`.
pub fn rewrite_font_families(css: &str, replace: impl Fn(&str) -> String) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(at) = find_property(rest) {
        let (head, tail) = rest.split_at(at);
        out.push_str(head);
        let colon = tail.find(':').map_or(tail.len(), |c| c + 1);
        out.push_str(&tail[..colon]);
        let len = value_end(&tail[colon..]);
        out.push_str(&replace(&tail[colon..colon + len]));
        rest = &tail[colon + len..];
    }
    out.push_str(rest);
    out
}

/// The length of a declaration value: up to `;` or `}` outside quotes.
fn value_end(s: &str) -> usize {
    let mut quote = None;
    for (i, c) in s.char_indices() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), c) if c == q => quote = None,
            (None, ';' | '}') => return i,
            _ => {}
        }
    }
    s.len()
}

fn find_property(css: &str) -> Option<usize> {
    let lower = css.to_ascii_lowercase();
    let mut from = 0;
    while let Some(i) = lower[from..].find("font-family") {
        let at = from + i;
        let before = lower[..at].chars().next_back();
        let after = lower[at + 11..].trim_start();
        if !before.is_some_and(|c| c.is_ascii_alphanumeric() || c == '-') && after.starts_with(':')
        {
            return Some(at);
        }
        from = at + 11;
    }
    None
}

fn strip_comments(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut rest = css;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        rest = rest[start..]
            .find("*/")
            .map_or("", |e| &rest[start + e + 2..]);
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_selectors_give_families() {
        let mut rules = Rules::default();
        rules.add(
            "<!--table {}\n.font12 {color:blue; font-family:\"BRH Sirigannada\";}\n\
             td.x, P {font-family: Arial} /* td {font-family: Nudi 01 e} */ div p {font-family: Y}",
        );
        assert_eq!(rules.family("font", "font12"), Some("\"BRH Sirigannada\""));
        assert_eq!(rules.family("td", "x"), Some("Arial"));
        assert_eq!(rules.family("p", ""), Some("Arial"));
        assert_eq!(rules.family("td", ""), None);
        assert_eq!(rules.family("div", ""), None);
    }

    #[test]
    fn declarations_and_rewrites() {
        assert_eq!(
            font_family("color:red; FONT-FAMILY: 'BRH Kannada'"),
            Some("'BRH Kannada'")
        );
        assert_eq!(font_family("font-size:10pt"), None);
        let css = "p{font-family:\"A;B\";mso-font-family:x} q{font-family: C}";
        let out = rewrite_font_families(css, |v| format!("[{v}]"));
        assert_eq!(
            out,
            "p{font-family:[\"A;B\"];mso-font-family:x} q{font-family:[ C]}"
        );
    }
}
