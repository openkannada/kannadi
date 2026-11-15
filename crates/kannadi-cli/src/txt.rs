//! Plain text files. The whole file is taken to be in the source encoding.

use std::collections::HashSet;

use anyhow::{bail, Result};
use kannadi::legacy::{decode_windows_1252, windows_1252_byte};

use crate::convert::{convert, describe_char, Direction, Issue, Kind, Position, Settings};
use crate::report::{Report, Row};

/// Converts a text file. Legacy text is read as UTF-8 when the bytes are valid
/// UTF-8, and as Windows-1252 otherwise; it is written as Windows-1252.
pub fn convert_file(bytes: &[u8], settings: &Settings) -> Result<(Vec<u8>, Report)> {
    match settings.direction {
        Direction::ToUnicode => {
            let text = match std::str::from_utf8(bytes) {
                Ok(s) => s.strip_prefix('\u{FEFF}').unwrap_or(s).to_string(),
                Err(_) => decode_windows_1252(bytes),
            };
            let converted = convert(&text, settings);
            let report = report(&text, &converted.text, &converted.text, &converted.issues);
            Ok((converted.text.into_bytes(), report))
        }
        Direction::ToNudi => {
            let Ok(text) = std::str::from_utf8(bytes) else {
                bail!("not UTF-8; Unicode input must be UTF-8");
            };
            let text = text.strip_prefix('\u{FEFF}').unwrap_or(text);
            let converted = convert(text, settings);
            let mut report = report(text, &converted.text, text, &converted.issues);
            let (out, lost) = to_windows_1252(&converted.text, &converted.issues);
            report.rows.extend(lost);
            Ok((out, report))
        }
    }
}

/// Encodes as Windows-1252, writing `?` for characters it has no byte for.
/// Those not already reported as unencodable get a row of their own.
fn to_windows_1252(text: &str, issues: &[Issue]) -> (Vec<u8>, Vec<Row>) {
    let reported: HashSet<&str> = issues.iter().map(|i| i.detail.as_str()).collect();
    let mut bytes = Vec::with_capacity(text.len());
    let mut rows = Vec::new();
    for (i, c) in text.char_indices() {
        match windows_1252_byte(c) {
            Some(b) => bytes.push(b),
            None => {
                bytes.push(b'?');
                let detail = describe_char(c);
                if !reported.contains(detail.as_str()) {
                    rows.push(Row {
                        location: format!("line {}", line_of(text, i)),
                        kind: Kind::Unencodable,
                        detail,
                        context: String::new(),
                    });
                }
            }
        }
    }
    (bytes, rows)
}

/// Rows for `issues`, with the matching line of the Unicode side as context.
fn report(input: &str, output: &str, unicode: &str, issues: &[Issue]) -> Report {
    let rows = issues
        .iter()
        .map(|issue| {
            let line = match &issue.at {
                Position::Input(i) => line_of(input, *i),
                Position::Output(r) => line_of(output, r.start),
            };
            Row {
                location: format!("line {line}"),
                kind: issue.kind,
                detail: issue.detail.clone(),
                context: unicode
                    .lines()
                    .nth(line - 1)
                    .unwrap_or_default()
                    .to_string(),
            }
        })
        .collect();
    Report { rows }
}

fn line_of(text: &str, at: usize) -> usize {
    text[..at].matches('\n').count() + 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use kannadi::legacy::Options;

    fn settings(direction: Direction) -> Settings {
        Settings {
            direction,
            options: Options::default(),
        }
    }

    #[test]
    fn windows_1252_and_utf8_input_both_convert() {
        let ansi = [0x50, 0xC0, 0xA3, 0xC0, 0xDF, 0x71, 0xC0];
        let (out, report) = convert_file(&ansi, &settings(Direction::ToUnicode)).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "ಕನ್ನಡ");
        assert!(report.is_empty());
        let utf8 = "\u{FEFF}PÀ£ÀßqÀ".as_bytes();
        let (out, _) = convert_file(utf8, &settings(Direction::ToUnicode)).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "ಕನ್ನಡ");
    }

    #[test]
    fn rows_name_the_line() {
        let (_, report) =
            convert_file(b"P\xC0\nP\xC0 \xF1", &settings(Direction::ToUnicode)).unwrap();
        assert_eq!(report.rows[0].location, "line 2");
        assert_eq!(report.rows[0].context, "ಕ \u{F1}");
    }

    #[test]
    fn to_nudi_writes_windows_1252() {
        let (out, report) = convert_file("ಕನ್ನಡ।".as_bytes(), &settings(Direction::ToNudi)).unwrap();
        assert_eq!(out, [0x50, 0xC0, 0xA3, 0xC0, 0xDF, 0x71, 0xC0, 0x86]);
        assert!(report.is_empty());
    }

    #[test]
    fn to_nudi_reports_each_lost_character_once() {
        let (out, report) = convert_file("ಕ ಌ ‐".as_bytes(), &settings(Direction::ToNudi)).unwrap();
        assert_eq!(out, b"P\xC0 ? ?");
        let details: Vec<&str> = report.rows.iter().map(|r| r.detail.as_str()).collect();
        assert_eq!(details, ["ಌ (U+0C8C)", "‐ (U+2010)"]);
    }

    #[test]
    fn to_nudi_rejects_invalid_utf8() {
        assert!(convert_file(b"\xC0", &settings(Direction::ToNudi)).is_err());
    }
}
