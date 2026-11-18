//! Plain text files. The whole file is taken to be in the source encoding.

use anyhow::{bail, Result};
use kannadi::legacy::{decode_windows_1252, is_unicode, windows_1252_byte};

use crate::convert::{convert, keep, Converted, Direction, Settings};
use crate::report::{Report, Row};

/// Converts a text file. Unicode output is UTF-8. Nudi output is Windows-1252,
/// with `?` for each character that has no byte, which is already reported as
/// unencodable.
pub fn convert_file(bytes: &[u8], settings: &Settings) -> Result<(Vec<u8>, Report)> {
    let text = read_text(bytes, settings.direction)?;
    let converted = match settings.direction {
        Direction::ToUnicode(_) => to_unicode_by_line(&text, settings),
        Direction::ToNudi => convert(&text, settings.text_layout, settings),
    };
    let rows = converted
        .issues
        .iter()
        .map(|issue| Row::new(format!("line {}", issue.line), issue))
        .collect();
    let out = match settings.direction {
        Direction::ToUnicode(_) => converted.text.into_bytes(),
        Direction::ToNudi => converted
            .text
            .chars()
            .map(|c| windows_1252_byte(c).unwrap_or(b'?'))
            .collect(),
    };
    Ok((out, Report { rows }))
}

/// Converts `text` to Unicode, keeping the lines that are Unicode already, whose
/// English letters would otherwise be read as glyphs. Runs of lines of 1 kind
/// are handled together; a line of ASCII alone goes with the whole file.
fn to_unicode_by_line(text: &str, settings: &Settings) -> Converted {
    let layout = settings.text_layout;
    let whole = is_unicode(text, layout);
    let mut runs: Vec<(bool, usize, usize)> = Vec::new();
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        // A line of ASCII alone, such as English or Nudi's own letters, follows the file.
        let unicode = if line.is_ascii() {
            whole
        } else {
            is_unicode(line, layout)
        };
        match runs.last_mut() {
            Some((kind, _, end)) if *kind == unicode => *end += line.len(),
            _ => runs.push((unicode, at, at + line.len())),
        }
        at += line.len();
    }
    if runs.len() <= 1 {
        return match runs.first() {
            Some((true, ..)) => keep(text),
            _ => convert(text, layout, settings),
        };
    }
    let mut out = Converted {
        text: String::with_capacity(text.len() * 2),
        issues: Vec::new(),
    };
    let mut lines_before = 0;
    for (unicode, start, end) in runs {
        let part = &text[start..end];
        let done = if unicode {
            keep(part)
        } else {
            convert(part, layout, settings)
        };
        out.text.push_str(&done.text);
        out.issues.extend(done.issues.into_iter().map(|mut issue| {
            issue.line += lines_before;
            issue
        }));
        lines_before += part.matches('\n').count();
    }
    out
}

/// Reads text with a UTF-8 or UTF-16 byte order mark, or valid UTF-8. Legacy text
/// that is none of these is read as Windows-1252; Unicode text must be one of them.
pub fn read_text(bytes: &[u8], direction: Direction) -> Result<String> {
    if let Some(utf16) = utf16(bytes) {
        return utf16;
    }
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    match (std::str::from_utf8(bytes), direction) {
        (Ok(s), _) => Ok(s.to_string()),
        (Err(_), Direction::ToUnicode(_)) => Ok(decode_windows_1252(bytes)),
        (Err(_), Direction::ToNudi) => bail!("not UTF-8 or UTF-16; Unicode input must be"),
    }
}

fn utf16(bytes: &[u8]) -> Option<Result<String>> {
    let little = match bytes {
        [0xFF, 0xFE, ..] => true,
        [0xFE, 0xFF, ..] => false,
        _ => return None,
    };
    let payload = &bytes[2..];
    if payload.len() % 2 == 1 {
        return Some(Err(anyhow::anyhow!("UTF-16 file ends in half a character")));
    }
    let units: Vec<u16> = (0..payload.len())
        .step_by(2)
        .map(|i| {
            let pair = [payload[i], payload[i + 1]];
            if little {
                u16::from_le_bytes(pair)
            } else {
                u16::from_be_bytes(pair)
            }
        })
        .collect();
    Some(String::from_utf16(&units).map_err(|_| anyhow::anyhow!("invalid UTF-16")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::Kind;
    use kannadi::legacy::Options;

    fn to_unicode() -> Settings {
        Settings::new(Direction::ToUnicode(Options::default()))
    }

    fn to_nudi() -> Settings {
        Settings::new(Direction::ToNudi)
    }

    #[test]
    fn unicode_files_are_kept_as_they_are() {
        for text in [
            "ಕನ್ನಡ ನಾಡು, Bengaluru 2026\r\nಎರಡನೇ ಸಾಲು\n",
            "\u{FEFF}ಕನ್ನಡ Kannada",
        ] {
            let (out, report) = convert_file(text.as_bytes(), &to_unicode()).unwrap();
            assert_eq!(
                String::from_utf8(out).unwrap(),
                text.trim_start_matches('\u{FEFF}')
            );
            assert!(report.is_empty(), "{:?}", report.rows);
        }
        let utf16: Vec<u8> = [0xFF, 0xFE]
            .into_iter()
            .chain("ಕನ್ನಡ".encode_utf16().flat_map(u16::to_le_bytes))
            .collect();
        let (out, report) = convert_file(&utf16, &to_unicode()).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "ಕನ್ನಡ");
        assert!(report.is_empty());
    }

    #[test]
    fn kept_unicode_files_still_get_typo_suggestions() {
        let text = "ಮಳೆ 30°C ಇದೆ\nನೈರೃತ್ಯ ದಿಕ್ಕು\n";
        let (out, report) = convert_file(text.as_bytes(), &to_unicode()).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), text);
        let rows: Vec<(&str, Kind)> = report
            .rows
            .iter()
            .map(|r| (r.location.as_str(), r.kind))
            .collect();
        assert_eq!(rows, [("line 2", Kind::LikelyTypo)]);
    }

    #[test]
    fn a_nudi_line_in_a_mostly_unicode_file_is_converted() {
        let text = "ಕರ್ನಾಟಕ ಸರ್ಕಾರದ ಸುತ್ತೋಲೆ: ಎಲ್ಲಾ ಶಾಲೆಗಳಲ್ಲಿ ಕನ್ನಡ ಕಡ್ಡಾಯ, 2026\n\
            PÀ£ÀßqÀ \u{F1}\nನೈರೃತ್ಯ, English\nOffice hours\n";
        let (out, report) = convert_file(text.as_bytes(), &to_unicode()).unwrap();
        assert_eq!(
            String::from_utf8(out).unwrap(),
            "ಕರ್ನಾಟಕ ಸರ್ಕಾರದ ಸುತ್ತೋಲೆ: ಎಲ್ಲಾ ಶಾಲೆಗಳಲ್ಲಿ ಕನ್ನಡ ಕಡ್ಡಾಯ, 2026\n\
            ಕನ್ನಡ \u{F1}\nನೈರೃತ್ಯ, English\nOffice hours\n"
        );
        let rows: Vec<(&str, Kind)> = report
            .rows
            .iter()
            .map(|r| (r.location.as_str(), r.kind))
            .collect();
        assert_eq!(
            rows,
            [("line 2", Kind::UnknownGlyph), ("line 3", Kind::LikelyTypo)]
        );
    }

    #[test]
    fn ascii_lines_in_a_nudi_file_are_converted() {
        let (out, _) = convert_file("¸Á»vÀå\nPÀ£ÀßqÀ\n".as_bytes(), &to_unicode()).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "ಸಾಹಿತ್ಯ\nಕನ್ನಡ\n");
        // Q is ಕಿ in Nudi: a line of ASCII alone.
        let (out, _) = convert_file("PÀ£ÀßqÀ\nQ\n".as_bytes(), &to_unicode()).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "ಕನ್ನಡ\nಕಿ\n");
    }

    #[test]
    fn a_file_with_legacy_and_unicode_text_is_converted() {
        let (out, report) = convert_file("PÀ£ÀßqÀ ನಾಡು".as_bytes(), &to_unicode()).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "ಕನ್ನಡ ನಾಡು");
        assert!(report.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn windows_1252_utf8_and_utf16_input_convert() {
        let ansi = [0x50, 0xC0, 0xA3, 0xC0, 0xDF, 0x71, 0xC0];
        let (out, report) = convert_file(&ansi, &to_unicode()).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "ಕನ್ನಡ");
        assert!(report.is_empty());
        let utf8 = "\u{FEFF}PÀ£ÀßqÀ".as_bytes();
        let (out, _) = convert_file(utf8, &to_unicode()).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "ಕನ್ನಡ");
        let mut utf16 = vec![0xFF, 0xFE];
        utf16.extend("PÀ".encode_utf16().flat_map(u16::to_le_bytes));
        let (out, _) = convert_file(&utf16, &to_unicode()).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "ಕ");
    }

    #[test]
    fn text_files_are_read_in_the_layout_asked_for() {
        let mut settings = to_unicode();
        settings.text_layout = kannadi::legacy::Layout::Akruti;
        let (out, report) = convert_file(b"d\xE2}\xE2", &settings).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "ಕನ");
        assert!(report.is_empty());
    }

    #[test]
    fn truncated_utf16_is_an_error() {
        for bytes in [&b"\xFF\xFE\x20"[..], b"\xFE\xFF\x00\x20\x00"] {
            assert!(convert_file(bytes, &to_unicode()).is_err());
        }
        let (out, _) = convert_file(b"\xFE\xFF\x00\x50\x00\xC0", &to_unicode()).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "ಕ");
    }

    #[test]
    fn empty_and_bom_only_files_convert_to_nothing() {
        for bytes in [&b""[..], b"\xEF\xBB\xBF", b"\xFF\xFE"] {
            let (out, report) = convert_file(bytes, &to_unicode()).unwrap();
            assert!(out.is_empty() && report.is_empty());
        }
    }

    #[test]
    fn rows_name_the_line_and_show_unicode_context() {
        let (_, report) = convert_file(b"P\xC0\r\nP\xC0 \xF1", &to_unicode()).unwrap();
        assert_eq!(report.rows[0].location, "line 2");
        assert_eq!(report.rows[0].kind, Kind::UnknownGlyph);
        assert_eq!(report.rows[0].context, "ಕ \u{F1}");
    }

    #[test]
    fn to_nudi_writes_windows_1252() {
        let (out, report) = convert_file("ಕನ್ನಡ।".as_bytes(), &to_nudi()).unwrap();
        assert_eq!(out, [0x50, 0xC0, 0xA3, 0xC0, 0xDF, 0x71, 0xC0, 0x86]);
        assert!(report.is_empty());
    }

    #[test]
    fn to_nudi_writes_a_question_mark_for_each_reported_character() {
        let (out, report) = convert_file("ಕ ಌ ‐".as_bytes(), &to_nudi()).unwrap();
        assert_eq!(out, b"P\xC0 ? ?");
        let details: Vec<&str> = report.rows.iter().map(|r| r.detail.as_str()).collect();
        assert_eq!(details, ["ಌ (U+0C8C)", "‐ (U+2010)"]);
    }

    #[test]
    fn to_nudi_rejects_windows_1252_input() {
        assert!(convert_file(b"\xC0", &to_nudi()).is_err());
    }
}
