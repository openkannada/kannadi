//! `wiki`: sentences from the held-out articles of a Kannada Wikipedia dump.
//!
//! Articles are split as kannada-spell's `kspell` splits them for its word counts: every
//! 10th article (namespace 0, no redirects) is held out. The protected-word list is
//! built from the other articles' counts, so no evaluation sentence went into it.

use std::io::BufRead;

const TEST_EVERY: usize = 10;

/// Sentences of the held-out articles, wiki markup removed, in dump order.
pub fn held_out_sentences(reader: impl BufRead) -> Vec<String> {
    let mut out = Vec::new();
    let mut page = String::new();
    let (mut article, mut redirect, mut in_text, mut articles) = (false, false, false, 0usize);
    for line in reader.lines() {
        let line = line.expect("read dump");
        let trimmed = line.trim_start();
        if trimmed.starts_with("<page>") {
            page.clear();
            (article, redirect, in_text) = (false, false, false);
        } else if trimmed.starts_with("<ns>") {
            article = trimmed.starts_with("<ns>0</ns>");
        } else if trimmed.starts_with("<redirect") {
            redirect = true;
        } else if trimmed.starts_with("</page>") {
            if article && !redirect {
                articles += 1;
                if articles % TEST_EVERY == 0 {
                    out.extend(sentences(&page));
                }
            }
            continue;
        }
        let mut text = line.as_str();
        if let Some(start) = text.find("<text") {
            in_text = true;
            text = text[start..].split_once('>').map_or("", |(_, rest)| rest);
        }
        if !in_text {
            continue;
        }
        if let Some(end) = text.find("</text>") {
            text = &text[..end];
            in_text = false;
        }
        page.push_str(text);
        page.push('\n');
    }
    out
}

fn sentences(wikitext: &str) -> Vec<String> {
    let text = unescape(wikitext);
    let text = remove_nested(&text, "{{", "}}");
    let text = remove_nested(&text, "{|", "|}");
    let text = remove_between(&text, "<ref", "</ref>");
    let text = remove_between(&text, "<!--", "-->");
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(['=', '|', '!', '*', '#', ':', ';', '<', '[']) {
            continue;
        }
        let line = links(line).replace("'''", "").replace("''", "");
        let line = remove_between(&line, "<", ">");
        for sentence in line.split(['.', '?', '!', '।']) {
            let sentence = sentence.split_whitespace().collect::<Vec<_>>().join(" ");
            if is_kannada_sentence(&sentence) {
                out.push(sentence);
            }
        }
    }
    out
}

/// 6 to 40 words, at least 80% of them Kannada, no markup left.
fn is_kannada_sentence(s: &str) -> bool {
    let words: Vec<&str> = s.split(' ').collect();
    let kannada = words.iter().filter(|w| w.chars().any(kannadi::chars::is_kannada)).count();
    (6..=40).contains(&words.len())
        && kannada * 5 >= words.len() * 4
        && !s.contains(['[', ']', '{', '}', '|', '=', '\t', '<', '>'])
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
}

fn remove_nested(s: &str, open: &str, close: &str) -> String {
    let mut out = String::new();
    let mut depth = 0usize;
    let mut i = 0;
    while i < s.len() {
        if s[i..].starts_with(open) {
            depth += 1;
            i += open.len();
        } else if depth > 0 && s[i..].starts_with(close) {
            depth -= 1;
            i += close.len();
        } else {
            let c = s[i..].chars().next().unwrap();
            if depth == 0 {
                out.push(c);
            }
            i += c.len_utf8();
        }
    }
    out
}

fn remove_between(s: &str, open: &str, close: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(start) = rest.find(open) {
        out.push_str(&rest[..start]);
        match rest[start..].find(close) {
            Some(end) => rest = &rest[start + end + close.len()..],
            None => {
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// [[target|label]] becomes label, [[target]] becomes target.
fn links(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(start) = rest.find("[[") {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find("]]") else {
            break;
        };
        let inner = &rest[start + 2..start + end];
        out.push_str(inner.rsplit('|').next().unwrap_or(inner));
        rest = &rest[start + end + 2..];
    }
    out.push_str(rest);
    out
}
