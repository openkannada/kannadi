//! Property tests for `.doc` and `.rtf` input: every run in a legacy font converts
//! to the text it was encoded from, and Unicode runs come out as they went in.

use std::fmt::Write as _;

use kannadi::legacy::{from_unicode, to_unicode, windows_1252_byte, Layout, Options};
use proptest::prelude::*;

use crate::convert::{Direction, Settings};
use crate::doc::build::{font, Doc, Piece, Style};
use crate::flow_writer::write;

const FONTS: [&str; 5] = [
    "Times New Roman",
    "Nudi 01 e",
    "BRH Kannada",
    "AkrutiKndPadma",
    "Noto Sans Kannada",
];

/// Kannada words from the fixture lines that each layout encodes and reads back.
fn words(layout: Layout) -> Vec<String> {
    let fixtures = [
        include_str!("../../kannadi/tests/fixtures/nudi.tsv"),
        include_str!("../../kannadi/tests/fixtures/akruti.tsv"),
    ];
    let mut out: Vec<String> = fixtures
        .iter()
        .flat_map(|f| f.lines())
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split('\t').nth(1))
        .flat_map(str::split_whitespace)
        .filter(|w| {
            let e = from_unicode(w, layout);
            e.unencodable.is_empty()
                && e.malformed.is_empty()
                && to_unicode(&e.text, layout, &Options::default()).text == *w
        })
        .map(String::from)
        .collect();
    out.sort();
    out.dedup();
    out
}

/// A run: its font, the text stored in the file, and the text expected back.
#[derive(Clone, Debug)]
struct Run {
    font: usize,
    stored: String,
    expected: String,
    /// A symbol font's codes, stored at U+F000 plus the code.
    symbol: bool,
}

fn legacy_run() -> impl Strategy<Value = Run> {
    let nudi = words(Layout::Nudi);
    let akruti = words(Layout::Akruti);
    (
        0..3usize,
        prop::collection::vec(any::<prop::sample::Index>(), 1..6),
        any::<bool>(),
    )
        .prop_map(move |(which, picks, symbol)| {
            let (font, list, layout) = match which {
                0 => (1, &nudi, Layout::Nudi),
                1 => (2, &nudi, Layout::Nudi),
                _ => (3, &akruti, Layout::Akruti),
            };
            let chosen: Vec<&String> = picks.iter().map(|i| i.get(list)).collect();
            let expected = chosen.iter().map(|w| format!("{w} ")).collect::<String>();
            let stored = chosen
                .iter()
                .map(|w| format!("{} ", from_unicode(w, layout).text))
                .collect();
            Run {
                font,
                stored,
                expected,
                symbol: symbol && font == 2,
            }
        })
}

fn unicode_run() -> impl Strategy<Value = Run> {
    let kannada = words(Layout::Nudi);
    let english = ["Office ", "2002 ", "No. 12/3 ", "Total: ", "{a\\b} "];
    (
        0..3usize,
        any::<prop::sample::Index>(),
        any::<prop::sample::Index>(),
    )
        .prop_map(move |(which, k, e)| {
            let (font, text) = match which {
                0 => (0, e.get(&english).to_string()),
                1 => (4, format!("{} ", k.get(&kannada))),
                // Unicode Kannada typed while a legacy font was chosen.
                _ => (1, format!("{} ", k.get(&kannada))),
            };
            Run {
                font,
                stored: text.clone(),
                expected: text,
                symbol: false,
            }
        })
}

fn paragraphs() -> impl Strategy<Value = Vec<Vec<Run>>> {
    let run = prop_oneof![legacy_run(), unicode_run()];
    prop::collection::vec(prop::collection::vec(run, 1..6), 1..5)
}

fn expected(paragraphs: &[Vec<Run>]) -> String {
    paragraphs
        .iter()
        .map(|p| {
            let line: String = p.iter().map(|r| r.expected.as_str()).collect();
            format!("{}\n", line)
        })
        .collect()
}

fn stored(run: &Run) -> String {
    if run.symbol {
        run.stored
            .chars()
            .map(|c| match windows_1252_byte(c) {
                Some(b) if b >= 0x20 => char::from_u32(0xF000 + u32::from(b)).unwrap(),
                _ => c,
            })
            .collect()
    } else {
        run.stored.clone()
    }
}

fn doc_bytes(paragraphs: &[Vec<Run>], compressed: &[bool]) -> Vec<u8> {
    let pieces = paragraphs
        .iter()
        .zip(compressed.iter().cycle())
        .map(|(p, &want)| {
            let texts: Vec<String> = p.iter().map(stored).collect();
            let fits = texts
                .iter()
                .flat_map(|t| t.chars())
                .all(|c| windows_1252_byte(c).is_some());
            let mut runs: Vec<(Vec<u8>, String)> = p
                .iter()
                .zip(texts)
                .map(|(r, t)| (font(r.font as u16), t))
                .collect();
            runs.last_mut().unwrap().1.push('\r');
            Piece {
                compressed: want && fits,
                grpprl: None,
                runs,
            }
        })
        .collect();
    Doc {
        fonts: FONTS.to_vec(),
        styles: vec![Style {
            kind: 1,
            base: 0x0FFF,
            grpprl: Vec::new(),
        }],
        pieces,
        ..Doc::default()
    }
    .bytes()
}

/// RTF with each run in its own group, and characters escaped in the style `escape` picks.
fn rtf_bytes(paragraphs: &[Vec<Run>], escape: &[u8]) -> Vec<u8> {
    let mut out = String::from("{\\rtf1\\ansi\\ansicpg1252\\deff0{\\fonttbl");
    for (i, name) in FONTS.iter().enumerate() {
        let charset = if i == 2 { 2 } else { 0 };
        let _ = write!(out, "{{\\f{i}\\fnil\\fcharset{charset} {name};}}");
    }
    out.push('}');
    let mut n = 0;
    for p in paragraphs {
        out.push_str("\\pard\\plain ");
        for run in p {
            let _ = write!(out, "{{\\f{} ", run.font);
            for c in stored(run).chars() {
                let style = escape[n % escape.len()] % 3;
                n += 1;
                match (c, windows_1252_byte(c)) {
                    ('\\' | '{' | '}', _) => {
                        out.push('\\');
                        out.push(c);
                    }
                    (' '..='~', _) if style != 2 => out.push(c),
                    (_, Some(b)) if style == 0 => {
                        let _ = write!(out, "\\'{b:02x}");
                    }
                    _ => {
                        let code = u32::from(c);
                        let signed = if code > 0x7FFF {
                            code as i32 - 65536
                        } else {
                            code as i32
                        };
                        let _ = write!(out, "\\u{signed}?");
                    }
                }
            }
            out.push('}');
        }
        out.push_str("\\par\n");
    }
    out.push('}');
    out.into_bytes()
}

fn text_of(document: &crate::flow::Document) -> (String, bool) {
    let settings = Settings::new(Direction::ToUnicode(Options::default()));
    let (out, report) = write(document, &settings, "test").unwrap();
    let text = String::from_utf8(out).unwrap();
    let clean = !report.has_problems();
    // Each line ends in the space after its last run, which the writer keeps.
    (text, clean)
}

/// 256 cases by default; set PROPTEST_CASES for a longer run.
fn cases() -> u32 {
    std::env::var("PROPTEST_CASES")
        .ok()
        .and_then(|n| n.parse().ok())
        .unwrap_or(256)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(cases()))]

    #[test]
    fn doc_runs_read_back(paragraphs in paragraphs(), compressed in prop::collection::vec(any::<bool>(), 1..4)) {
        let document = crate::doc::read(&doc_bytes(&paragraphs, &compressed)).unwrap();
        let (text, clean) = text_of(&document);
        prop_assert_eq!(text, expected(&paragraphs));
        prop_assert!(clean);
    }

    #[test]
    fn rtf_runs_read_back(paragraphs in paragraphs(), escape in prop::collection::vec(any::<u8>(), 1..8)) {
        let document = crate::rtf::read(&rtf_bytes(&paragraphs, &escape)).unwrap();
        let (text, clean) = text_of(&document);
        prop_assert_eq!(text, expected(&paragraphs));
        prop_assert!(clean);
    }
}

#[test]
fn each_layout_has_words_to_draw_from() {
    assert!(words(Layout::Nudi).len() > 50);
    assert!(words(Layout::Akruti).len() > 50);
}
