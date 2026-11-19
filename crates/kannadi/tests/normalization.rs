//! The test vectors of the normalization profile, in `normalization/`.

use std::fmt::Write;

use kannadi::normalize::{normalize_counted, Form, Rule};

const TSV: &str = include_str!("../../../normalization/vectors.tsv");
const JSON_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../normalization/vectors.json"
);

struct Vector {
    id: String,
    form: Form,
    rules: Vec<String>,
    input: String,
    expected: String,
    note: String,
}

/// Decodes `{ZWNJ}`, `{ZWJ}` and `{U+XXXX}`.
fn decode(field: &str) -> String {
    let mut out = String::new();
    let mut rest = field;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let close = rest[open..].find('}').expect("closing brace") + open;
        let name = &rest[open + 1..close];
        out.push(match name {
            "ZWNJ" => '\u{200C}',
            "ZWJ" => '\u{200D}',
            _ => {
                let hex = name.strip_prefix("U+").expect("{ZWNJ}, {ZWJ} or {U+XXXX}");
                char::from_u32(u32::from_str_radix(hex, 16).unwrap()).unwrap()
            }
        });
        rest = &rest[close + 1..];
    }
    out.push_str(rest);
    out
}

fn vectors() -> Vec<Vector> {
    TSV.lines()
        .filter(|l| !l.starts_with('#'))
        .skip(1)
        .map(|line| {
            let cols: Vec<&str> = line.split('\t').collect();
            assert_eq!(cols.len(), 6, "{line}");
            Vector {
                id: cols[0].to_string(),
                form: Form::from_name(cols[1]).expect("form"),
                rules: match cols[2] {
                    "-" => Vec::new(),
                    list => list.split(',').map(str::to_string).collect(),
                },
                input: decode(cols[3]),
                expected: decode(cols[4]),
                note: cols[5].to_string(),
            }
        })
        .collect()
}

#[test]
fn every_vector_normalizes_as_expected_with_its_rules() {
    let vectors = vectors();
    assert!(vectors.len() >= 60);
    for v in &vectors {
        let (got, counts) = normalize_counted(&v.input, v.form);
        assert_eq!(got, v.expected, "{}", v.id);
        let fired: Vec<&str> = Rule::ALL
            .into_iter()
            .filter(|&r| counts.get(r) > 0)
            .map(Rule::id)
            .collect();
        assert_eq!(fired, v.rules, "{}", v.id);
        let (again, counts) = normalize_counted(&got, v.form);
        assert_eq!(
            (again, counts.total()),
            (got, 0),
            "{} is not idempotent",
            v.id
        );
    }
}

#[test]
fn ids_are_unique_and_every_rule_has_a_vector() {
    let vectors = vectors();
    let mut ids: Vec<&str> = vectors.iter().map(|v| v.id.as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    assert_eq!(ids.len(), vectors.len());
    for rule in Rule::ALL {
        assert!(
            vectors
                .iter()
                .any(|v| v.rules.iter().any(|r| r == rule.id())),
            "no vector for {}",
            rule.id()
        );
    }
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 || matches!(c, '\u{200B}'..='\u{200D}' | '\u{00AD}') => {
                write!(out, "\\u{:04x}", c as u32).unwrap()
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn codepoints(s: &str) -> String {
    let list: Vec<String> = s
        .chars()
        .map(|c| format!("\"U+{:04X}\"", c as u32))
        .collect();
    format!("[{}]", list.join(", "))
}

/// vectors.json, written from vectors.tsv: the same vectors, with plain strings and
/// code point lists.
fn vectors_json() -> String {
    let mut out = String::from("[\n");
    let vectors = vectors();
    for (i, v) in vectors.iter().enumerate() {
        let rules: Vec<String> = v.rules.iter().map(|r| json_string(r)).collect();
        writeln!(out, "  {{").unwrap();
        writeln!(out, "    \"id\": {},", json_string(&v.id)).unwrap();
        writeln!(out, "    \"form\": {},", json_string(v.form.name())).unwrap();
        writeln!(out, "    \"rules\": [{}],", rules.join(", ")).unwrap();
        writeln!(out, "    \"input\": {},", json_string(&v.input)).unwrap();
        writeln!(out, "    \"expected\": {},", json_string(&v.expected)).unwrap();
        writeln!(out, "    \"input_codepoints\": {},", codepoints(&v.input)).unwrap();
        writeln!(
            out,
            "    \"expected_codepoints\": {},",
            codepoints(&v.expected)
        )
        .unwrap();
        writeln!(out, "    \"note\": {}", json_string(&v.note)).unwrap();
        let comma = if i + 1 < vectors.len() { "," } else { "" };
        writeln!(out, "  }}{comma}").unwrap();
    }
    out.push_str("]\n");
    out
}

/// Rewrite vectors.json with `KANNADI_BLESS=1 cargo test --test normalization`.
#[test]
fn vectors_json_matches_the_tsv() {
    let fresh = vectors_json();
    if std::env::var_os("KANNADI_BLESS").is_some() {
        std::fs::write(JSON_PATH, &fresh).unwrap();
        return;
    }
    let committed = std::fs::read_to_string(JSON_PATH).unwrap_or_default();
    assert!(
        committed == fresh,
        "normalization/vectors.json is out of date; rerun with KANNADI_BLESS=1"
    );
}
