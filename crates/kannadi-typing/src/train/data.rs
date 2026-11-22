//! Readers for the training and test data.

use crate::roman::letters;
use crate::units::{text, units, Unit};
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

fn lines(path: &Path) -> io::Result<impl Iterator<Item = String>> {
    Ok(BufReader::new(File::open(path)?)
        .lines()
        .map_while(Result::ok))
}

/// The string value of `"field": "..."` in 1 line of JSON, unescaping only `\"` and `\\`.
fn json_field<'a>(line: &'a str, field: &str) -> Option<std::borrow::Cow<'a, str>> {
    let at = line.find(&format!("\"{field}\""))? + field.len() + 2;
    let rest = line[at..]
        .trim_start()
        .strip_prefix(':')?
        .trim_start()
        .strip_prefix('"')?;
    let mut end = 0;
    let bytes = rest.as_bytes();
    while end < bytes.len() && bytes[end] != b'"' {
        end += if bytes[end] == b'\\' { 2 } else { 1 };
    }
    let raw = rest.get(..end)?;
    Some(if raw.contains('\\') {
        raw.replace("\\\"", "\"").replace("\\\\", "\\").into()
    } else {
        raw.into()
    })
}

/// 1 Aksharantar row.
pub struct AksharantarRow {
    pub native: String,
    pub roman: String,
    pub source: String,
}

/// Reads an Aksharantar JSON-lines file.
pub fn aksharantar(path: &Path) -> io::Result<Vec<AksharantarRow>> {
    Ok(lines(path)?
        .filter_map(|l| {
            Some(AksharantarRow {
                native: json_field(&l, "native word")?.into_owned(),
                roman: json_field(&l, "english word")?.into_owned(),
                source: json_field(&l, "source")?.into_owned(),
            })
        })
        .collect())
}

/// Reads a Dakshina lexicon: native word, romanization, number of annotators.
pub fn dakshina(path: &Path) -> io::Result<Vec<(String, String)>> {
    Ok(lines(path)?
        .filter_map(|l| {
            let mut f = l.split('\t');
            Some((f.next()?.to_string(), f.next()?.to_string()))
        })
        .collect())
}

/// Reads a word count list: word, tab, count; lines starting with # are comments.
pub fn counts(path: &Path) -> io::Result<Vec<(String, u64)>> {
    Ok(lines(path)?
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| {
            let (w, c) = l.split_once('\t')?;
            Some((w.to_string(), c.trim().parse().ok()?))
        })
        .collect())
}

/// Kannada words of a line of text. A token that is not a Kannada word splits the line.
pub fn words(line: &str) -> Vec<Vec<String>> {
    let mut out = vec![Vec::new()];
    for token in line.split_whitespace() {
        let token =
            token.trim_matches(|c: char| c.is_ascii_punctuation() || "“”‘’।॥…–—".contains(c));
        match units(token) {
            Some(u) => out.last_mut().unwrap().push(text(&u)),
            None => out.push(Vec::new()),
        }
    }
    out.retain(|s| !s.is_empty());
    out
}

/// Reads sentences, 1 per line, taking the text after the last tab.
pub fn sentences(path: &Path, limit: usize) -> io::Result<Vec<Vec<String>>> {
    Ok(lines(path)?
        .take(limit)
        .flat_map(|l| words(l.rsplit('\t').next().unwrap_or("")))
        .collect())
}

/// A training pair, or `None` if either side is not usable.
pub fn pair(roman: &str, native: &str) -> Option<(Vec<u8>, Vec<Unit>)> {
    let r = letters(roman)?;
    let u = units(native)?;
    (!r.is_empty()).then_some((r, u))
}

/// The native words of a test set, for keeping them out of training.
pub fn natives(rows: &[(String, String)]) -> HashSet<String> {
    rows.iter()
        .filter_map(|(n, _)| units(n).map(|u| text(&u)))
        .collect()
}
