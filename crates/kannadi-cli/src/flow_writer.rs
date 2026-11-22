//! Writes a [`Document`] read from a `.doc` or `.rtf` file as UTF-8 text, 1 line per
//! paragraph and a blank line between stories. Runs in a legacy font are converted
//! in that font's layout; other runs are kept. Report rows give the output line.

use std::collections::BTreeMap;

use anyhow::{bail, Result};
use kannadi::legacy::{draws_kannada_digits, is_legacy_font, windows_1252_byte, Layout, Options};

use crate::convert::{convert, Direction, Kind, Settings};
use crate::flow::{glyph_code, Document};
use crate::report::{Report, Row};

/// How a run's text is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reading {
    /// In a layout, with Kannada digits at the ASCII digit codes or not.
    Legacy(Layout, bool),
    /// A legacy font with no table: left out and reported.
    NoTable,
    Other,
}

pub fn write(doc: &Document, settings: &Settings, format: &str) -> Result<(Vec<u8>, Report)> {
    let Direction::ToUnicode(options) = settings.direction else {
        bail!("{format} files can only be converted to Unicode");
    };
    let readings: Vec<Reading> = doc
        .fonts
        .iter()
        .map(|f| reading(&f.name, settings))
        .collect();
    let writer = Writer {
        latin_digits: with_digits(settings, options, false),
        kannada_digits: with_digits(settings, options, true),
    };
    let mut out = String::new();
    let mut report = Report::default();
    let mut left_out: BTreeMap<&str, usize> = BTreeMap::new();
    let mut line = 0;
    for (i, story) in doc.stories.iter().enumerate() {
        if i > 0 {
            out.push('\n');
            line += 1;
        }
        for paragraph in &story.paragraphs {
            line += 1;
            let mut text = Line::new(format!("line {line}"));
            for run in paragraph {
                match run.font.and_then(|f| readings.get(f)) {
                    Some(&Reading::Legacy(layout, digits)) => {
                        for c in run.text.chars() {
                            let code = glyph_code(c);
                            if windows_1252_byte(code).is_some() {
                                text.legacy(code, (layout, digits), &writer, &mut report);
                            } else {
                                text.keep(c, &writer, &mut report);
                            }
                        }
                    }
                    Some(Reading::NoTable) => {
                        text.flush(&writer, &mut report);
                        let name = &doc.fonts[run.font.unwrap_or_default()].name;
                        *left_out.entry(name).or_default() += run.text.chars().count();
                    }
                    _ => run
                        .text
                        .chars()
                        .for_each(|c| text.keep(c, &writer, &mut report)),
                }
            }
            text.flush(&writer, &mut report);
            out.push_str(&text.out);
            out.push('\n');
        }
    }
    for (font, count) in left_out {
        report.rows.push(Row {
            location: format!("font {font}"),
            kind: Kind::NotConverted,
            detail: format!(
                "{count} characters in a legacy font whose layout this tool has no table for, \
                 left out"
            ),
            context: String::new(),
        });
    }
    Ok((out.into_bytes(), report))
}

fn reading(font: &str, settings: &Settings) -> Reading {
    match settings.fonts.layout(font) {
        Some(layout) => Reading::Legacy(layout, draws_kannada_digits(font)),
        None if is_legacy_font(font) => Reading::NoTable,
        None => Reading::Other,
    }
}

fn with_digits(settings: &Settings, options: Options, kannada_digits: bool) -> Settings {
    Settings {
        direction: Direction::ToUnicode(Options {
            kannada_digits: options.kannada_digits || kannada_digits,
            ..options
        }),
        ..settings.clone()
    }
}

struct Writer {
    /// For text in the `e` fonts, and in the `k` fonts, which draw Kannada digits.
    latin_digits: Settings,
    kannada_digits: Settings,
}

/// 1 output line: kept text, and legacy text gathered until its reading changes.
struct Line {
    location: String,
    out: String,
    legacy: String,
    reading: (Layout, bool),
}

impl Line {
    fn new(location: String) -> Self {
        Line {
            location,
            out: String::new(),
            legacy: String::new(),
            reading: (Layout::Nudi, false),
        }
    }

    fn legacy(&mut self, code: char, reading: (Layout, bool), w: &Writer, report: &mut Report) {
        if reading != self.reading {
            self.flush(w, report);
            self.reading = reading;
        }
        self.legacy.push(code);
    }

    fn keep(&mut self, c: char, w: &Writer, report: &mut Report) {
        self.flush(w, report);
        self.out.push(c);
    }

    fn flush(&mut self, w: &Writer, report: &mut Report) {
        if self.legacy.is_empty() {
            return;
        }
        let (layout, digits) = self.reading;
        let settings = if digits {
            &w.kannada_digits
        } else {
            &w.latin_digits
        };
        let converted = convert(&self.legacy, layout, settings);
        report.rows.extend(
            converted
                .issues
                .iter()
                .map(|issue| Row::new(self.location.clone(), issue)),
        );
        self.out.push_str(&converted.text);
        self.legacy.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::{Font, Run, Story};

    fn doc(fonts: &[&str], paragraphs: Vec<Vec<(Option<usize>, &str)>>) -> Document {
        let mut story = Story::new("body");
        story.paragraphs = paragraphs
            .into_iter()
            .map(|p| {
                p.into_iter()
                    .map(|(font, text)| Run {
                        font,
                        text: text.to_string(),
                    })
                    .collect()
            })
            .collect();
        Document {
            fonts: fonts
                .iter()
                .map(|n| Font {
                    name: n.to_string(),
                })
                .collect(),
            stories: vec![story],
        }
    }

    fn run(doc: &Document) -> (String, Report) {
        let settings = Settings::new(Direction::ToUnicode(Options::default()));
        let (out, report) = write(doc, &settings, "Word").unwrap();
        (String::from_utf8(out).unwrap(), report)
    }

    const FONTS: [&str; 6] = [
        "Times New Roman",
        "Nudi 01 e",
        "BRH Kannada",
        "AkrutiKndPadma",
        "Shree-Kan-0851",
        "Nudi 04 k",
    ];

    #[test]
    fn legacy_runs_convert_in_their_layout_and_others_stay() {
        let d = doc(
            &FONTS,
            vec![vec![
                (Some(1), "PÀ"),
                (Some(0), " Office 12 "),
                (Some(3), "dâ}â"),
                (None, " PÀ"),
            ]],
        );
        let (text, report) = run(&d);
        assert_eq!(text, "ಕ Office 12 ಕನ PÀ\n");
        assert!(report.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn a_word_split_across_runs_of_1_layout_converts_whole() {
        // ಕಿ is P then the vowel sign at C2 in Nudi; Baraha shares the layout.
        let d = doc(&FONTS, vec![vec![(Some(1), "P"), (Some(2), "Â")]]);
        let (text, report) = run(&d);
        assert_eq!(text, "ಕಿ\n");
        assert!(report.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn unicode_in_a_legacy_run_is_kept_and_symbol_codes_are_read() {
        let d = doc(
            &FONTS,
            vec![vec![(Some(1), "\u{F050}\u{F0C0} ಕನ್ನಡ ā क PÀ")]],
        );
        let (text, report) = run(&d);
        assert_eq!(text, "ಕ ಕನ್ನಡ ā क ಕ\n");
        assert!(report.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn k_fonts_draw_kannada_digits() {
        let d = doc(&FONTS, vec![vec![(Some(5), "12")], vec![(Some(1), "12")]]);
        assert_eq!(run(&d).0, "೧೨\n12\n");
    }

    #[test]
    fn problems_are_reported_by_output_line_and_no_table_fonts_left_out() {
        let mut d = doc(
            &FONTS,
            vec![vec![(Some(0), "English")], vec![(Some(1), "PÀ Á")]],
        );
        let mut notes = Story::new("footnotes");
        notes.paragraphs = vec![vec![
            Run {
                font: Some(4),
                text: "abc".into(),
            },
            Run {
                font: Some(1),
                text: "\u{F1}".into(),
            },
        ]];
        d.stories.push(notes);
        let (text, report) = run(&d);
        assert_eq!(text, "English\nಕ ಾ\n\n\u{F1}\n");
        let rows: Vec<(&str, Kind)> = report
            .rows
            .iter()
            .map(|r| (r.location.as_str(), r.kind))
            .collect();
        assert_eq!(
            rows,
            [
                ("line 2", Kind::Malformed),
                ("line 4", Kind::UnknownGlyph),
                ("font Shree-Kan-0851", Kind::NotConverted),
            ]
        );
        assert!(report.rows[2].detail.starts_with("3 characters"));
    }

    #[test]
    fn added_legacy_fonts_are_read_in_their_layout() {
        let d = doc(&["House Font"], vec![vec![(Some(0), "dâ}â")]]);
        let mut settings = Settings::new(Direction::ToUnicode(Options::default()));
        settings.fonts.extra_legacy = vec![("house font".into(), Layout::Akruti)];
        let (out, _) = write(&d, &settings, "Word").unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "ಕನ\n");
    }

    #[test]
    fn to_nudi_is_refused() {
        let settings = Settings::new(Direction::ToNudi);
        let err = write(&doc(&FONTS, Vec::new()), &settings, "RTF").unwrap_err();
        assert!(err.to_string().contains("RTF files can only"), "{err}");
    }
}
