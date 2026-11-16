//! 1 XML part of a DOCX: its paragraphs, runs and text, found by position so the
//! part can be edited in place.

use std::ops::Range;

use anyhow::Result;
use kannadi::akshara::aksharas;
use kannadi::legacy::nudi_syllable_starts;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::convert::{convert, kannada_runs, Direction, Fonts, Issue, Settings};
use crate::edits::apply;
use crate::report::Row;

use super::styles::{attr, legacy_font, Styles};
use super::xml;

#[derive(Debug, Default)]
struct Paragraph {
    /// 1-based, in document order.
    number: usize,
    style: Option<String>,
    runs: Vec<Run>,
}

#[derive(Debug, Default)]
struct Run {
    /// Where the run's content starts, just after `<w:r ...>`.
    content_start: usize,
    properties: Option<Properties>,
    pieces: Vec<Piece>,
}

/// A run's `w:rPr`.
#[derive(Debug, Default)]
struct Properties {
    span: Range<usize>,
    /// Written as `<w:rPr/>`.
    empty: bool,
    /// Just after `<w:rPr>`.
    content_start: usize,
    style: Option<String>,
    /// Just after `<w:rStyle/>`, where `w:rFonts` must go.
    after_style: Option<usize>,
    fonts: Option<(Range<usize>, Option<bool>)>,
    /// The `w:sz` value and the position just after it.
    size: Option<(String, usize)>,
    complex_size: bool,
}

#[derive(Debug)]
enum Piece {
    Text {
        tag: Range<usize>,
        content: Range<usize>,
    },
    /// A tab or line break, which no syllable crosses.
    Break,
}

/// Converts the text of 1 part. Returns the new part, or `None` if nothing changed.
pub fn convert_part(
    name: &str,
    source: &str,
    styles: &Styles,
    settings: &Settings,
    rows: &mut Vec<Row>,
) -> Result<Option<String>> {
    let paragraphs = scan(source, &settings.fonts)?;
    let mut edits = Vec::new();
    for paragraph in &paragraphs {
        let legacy: Vec<bool> = paragraph
            .runs
            .iter()
            .map(|r| is_legacy(r, paragraph, styles))
            .collect();
        let wanted = matches!(settings.direction, Direction::ToUnicode(_));
        let mut converted_runs = vec![false; paragraph.runs.len()];
        for group in groups(paragraph, &legacy, wanted) {
            let location = format!("{name}, paragraph {}", paragraph.number);
            convert_group(
                source, paragraph, &group, settings, &location, &mut edits, rows,
            );
            for &(run, _) in &group {
                converted_runs[run] = true;
            }
        }
        if let Direction::ToUnicode(_) = settings.direction {
            for (run, _) in paragraph
                .runs
                .iter()
                .zip(&converted_runs)
                .filter(|(_, &c)| c)
            {
                let font = &settings.fonts.unicode;
                let properties = properties_with_font(source, run.properties.as_ref(), font, true);
                match &run.properties {
                    Some(p) => edits.push((p.span.clone(), properties)),
                    None => edits.push((run.content_start..run.content_start, properties)),
                }
            }
        }
    }
    if edits.is_empty() {
        return Ok(None);
    }
    Ok(Some(apply(source, edits)))
}

/// Runs of consecutive text pieces whose runs are `wanted` legacy or not, split at
/// tabs, breaks and runs of the other kind. Each item is (run index, piece index).
fn groups(paragraph: &Paragraph, legacy: &[bool], wanted: bool) -> Vec<Vec<(usize, usize)>> {
    let mut groups = Vec::new();
    let mut current = Vec::new();
    for (r, run) in paragraph.runs.iter().enumerate() {
        if legacy[r] != wanted {
            groups.push(std::mem::take(&mut current));
            continue;
        }
        for (p, piece) in run.pieces.iter().enumerate() {
            match piece {
                Piece::Text { .. } => current.push((r, p)),
                Piece::Break => groups.push(std::mem::take(&mut current)),
            }
        }
    }
    groups.push(current);
    groups.retain(|g| !g.is_empty());
    groups
}

fn convert_group(
    source: &str,
    paragraph: &Paragraph,
    group: &[(usize, usize)],
    settings: &Settings,
    location: &str,
    edits: &mut Vec<(Range<usize>, String)>,
    rows: &mut Vec<Row>,
) {
    let spans: Vec<(&Range<usize>, &Range<usize>)> = group
        .iter()
        .filter_map(|&(r, p)| match &paragraph.runs[r].pieces[p] {
            Piece::Text { tag, content } => Some((tag, content)),
            Piece::Break => None,
        })
        .collect();
    let texts: Vec<String> = spans
        .iter()
        .map(|(_, c)| xml::decode(&source[(*c).clone()]))
        .collect();
    let texts = rebalance(&texts, settings.direction);
    for (i, ((tag, content), text)) in spans.iter().zip(&texts).enumerate() {
        let original = xml::decode(&source[(*content).clone()]);
        let new = match settings.direction {
            Direction::ToUnicode(_) => {
                let converted = convert(text, settings);
                add_rows(rows, location, &converted.issues);
                xml::escape(&converted.text)
            }
            Direction::ToNudi => {
                let run = &paragraph.runs[group[i].0];
                match legacy_splice(source, run, text, settings, location, rows) {
                    Some(spliced) => spliced,
                    None if *text == original => continue,
                    None => xml::escape(text),
                }
            }
        };
        edits.push(((*tag).clone(), "<w:t xml:space=\"preserve\">".to_string()));
        edits.push(((*content).clone(), new));
    }
}

/// Moves the text at each piece boundary that falls inside a syllable to the
/// piece where the syllable starts, so no syllable is split across runs.
fn rebalance(texts: &[String], direction: Direction) -> Vec<String> {
    let joined = texts.concat();
    let starts: Vec<usize> = match direction {
        Direction::ToUnicode(_) => nudi_syllable_starts(&joined),
        Direction::ToNudi => {
            let mut at = 0;
            aksharas(&joined)
                .map(|a| {
                    let start = at;
                    at += a.text.len();
                    start
                })
                .collect()
        }
    };
    let mut bounds = vec![0];
    let mut end = 0;
    for text in &texts[..texts.len().saturating_sub(1)] {
        end += text.len();
        let next = starts
            .iter()
            .copied()
            .find(|&s| s >= end)
            .unwrap_or(joined.len());
        bounds.push(next.max(*bounds.last().unwrap_or(&0)));
    }
    bounds.push(joined.len());
    bounds
        .windows(2)
        .map(|w| joined[w[0]..w[1]].to_string())
        .collect()
}

/// The content of a `w:t` with each Kannada run moved into a new run set in the
/// legacy font, or `None` if `text` has no Kannada.
fn legacy_splice(
    source: &str,
    run: &Run,
    text: &str,
    settings: &Settings,
    location: &str,
    rows: &mut Vec<Row>,
) -> Option<String> {
    let runs = kannada_runs(text);
    if runs.is_empty() {
        return None;
    }
    let own = run
        .properties
        .as_ref()
        .map_or("", |p| &source[p.span.clone()]);
    let legacy = properties_with_font(
        source,
        run.properties.as_ref(),
        &settings.fonts.legacy,
        false,
    );
    let mut out = String::new();
    let mut last = 0;
    for range in runs {
        let converted = convert(&text[range.clone()], settings);
        add_rows(rows, location, &converted.issues);
        out.push_str(&xml::escape(&text[last..range.start]));
        out.push_str("</w:t></w:r><w:r>");
        out.push_str(&legacy);
        out.push_str("<w:t xml:space=\"preserve\">");
        out.push_str(&xml::escape(&converted.text));
        out.push_str("</w:t></w:r><w:r>");
        out.push_str(own);
        out.push_str("<w:t xml:space=\"preserve\">");
        last = range.end;
    }
    out.push_str(&xml::escape(&text[last..]));
    Some(out)
}

/// The run's `w:rPr` with its fonts set to `font`. For text converted to Unicode,
/// a size for complex scripts is added where the run sets only `w:sz`, since Word
/// sizes Kannada with `w:szCs`.
fn properties_with_font(
    source: &str,
    properties: Option<&Properties>,
    font: &str,
    complex: bool,
) -> String {
    let f = xml::escape(font);
    let fonts =
        format!("<w:rFonts w:ascii=\"{f}\" w:hAnsi=\"{f}\" w:eastAsia=\"{f}\" w:cs=\"{f}\"/>");
    let Some(p) = properties.filter(|p| !p.empty) else {
        return format!("<w:rPr>{fonts}</w:rPr>");
    };
    let base = p.span.start;
    let mut edits = Vec::new();
    match &p.fonts {
        Some((span, _)) => edits.push((span.start - base..span.end - base, fonts)),
        None => {
            let at = p.after_style.unwrap_or(p.content_start) - base;
            edits.push((at..at, fonts));
        }
    }
    if let (true, Some((size, end)), false) = (complex, &p.size, p.complex_size) {
        let at = end - base;
        edits.push((at..at, format!("<w:szCs w:val=\"{}\"/>", xml::escape(size))));
    }
    apply(&source[p.span.clone()], edits)
}

/// Whether a run is set in a legacy font: by its own fonts, then its character
/// style, then its paragraph style, then the document default.
fn is_legacy(run: &Run, paragraph: &Paragraph, styles: &Styles) -> bool {
    let p = run.properties.as_ref();
    let direct = p
        .and_then(|p| p.fonts.as_ref())
        .and_then(|(_, legacy)| *legacy);
    let character = || {
        p.and_then(|p| p.style.as_deref())
            .and_then(|s| styles.legacy(s))
    };
    let paragraph = || match &paragraph.style {
        Some(s) => styles.legacy(s),
        None => styles.default_paragraph(),
    };
    direct
        .or_else(character)
        .or_else(paragraph)
        .or_else(|| styles.document_default())
        .unwrap_or(false)
}

fn add_rows(rows: &mut Vec<Row>, location: &str, issues: &[Issue]) {
    rows.extend(
        issues
            .iter()
            .map(|issue| Row::new(location.to_string(), issue)),
    );
}

/// Finds paragraphs, runs and text in document order of their end, which puts
/// paragraphs inside text boxes before the paragraph that holds the text box.
fn scan(source: &str, fonts: &Fonts) -> Result<Vec<Paragraph>> {
    let mut reader = Reader::from_str(source);
    let mut names: Vec<String> = Vec::new();
    let mut paragraphs: Vec<Paragraph> = Vec::new();
    let mut runs: Vec<Run> = Vec::new();
    let mut done = Vec::new();
    let mut count = 0;
    let mut text: Option<(Range<usize>, usize)> = None;
    loop {
        let start = position(&reader);
        let event = reader.read_event()?;
        let end = position(&reader);
        let parent = names.last().map(String::as_str);
        let grandparent = names.len().checked_sub(2).map(|i| names[i].as_str());
        match event {
            Event::Eof => break,
            Event::Start(e) => {
                let mut state = State {
                    paragraphs: &mut paragraphs,
                    runs: &mut runs,
                    count: &mut count,
                    text: &mut text,
                    fonts,
                };
                open(&e, start..end, false, parent, grandparent, &mut state);
                names.push(e.name().as_ref().to_string());
            }
            Event::Empty(e) => {
                let mut state = State {
                    paragraphs: &mut paragraphs,
                    runs: &mut runs,
                    count: &mut count,
                    text: &mut text,
                    fonts,
                };
                open(&e, start..end, true, parent, grandparent, &mut state);
                close(
                    e.name().as_ref(),
                    end,
                    parent,
                    &mut paragraphs,
                    &mut runs,
                    &mut done,
                );
            }
            Event::End(e) => {
                names.pop();
                let parent = names.last().map(String::as_str);
                if e.name().as_ref() == "w:t" {
                    if let (Some((tag, content_start)), Some(run)) = (text.take(), runs.last_mut())
                    {
                        run.pieces.push(Piece::Text {
                            tag,
                            content: content_start..start,
                        });
                    }
                }
                close(
                    e.name().as_ref(),
                    end,
                    parent,
                    &mut paragraphs,
                    &mut runs,
                    &mut done,
                );
            }
            _ => {}
        }
    }
    Ok(done)
}

struct State<'a> {
    paragraphs: &'a mut Vec<Paragraph>,
    runs: &'a mut Vec<Run>,
    count: &'a mut usize,
    text: &'a mut Option<(Range<usize>, usize)>,
    fonts: &'a Fonts,
}

fn open(
    e: &BytesStart,
    span: Range<usize>,
    empty: bool,
    parent: Option<&str>,
    grandparent: Option<&str>,
    state: &mut State,
) {
    let in_properties = parent == Some("w:rPr") && grandparent == Some("w:r");
    match e.name().as_ref() {
        "w:p" => {
            *state.count += 1;
            state.paragraphs.push(Paragraph {
                number: *state.count,
                ..Paragraph::default()
            });
        }
        "w:pStyle" if parent == Some("w:pPr") => {
            if let Some(p) = state.paragraphs.last_mut() {
                p.style = attr(e, "w:val");
            }
        }
        "w:r" => state.runs.push(Run {
            content_start: span.end,
            ..Run::default()
        }),
        "w:rPr" if parent == Some("w:r") => {
            if let Some(run) = state.runs.last_mut() {
                run.properties = Some(Properties {
                    span: span.clone(),
                    empty,
                    content_start: span.end,
                    ..Properties::default()
                });
            }
        }
        "w:t" if parent == Some("w:r") && !empty => *state.text = Some((span.clone(), span.end)),
        "w:tab" | "w:br" | "w:cr" if parent == Some("w:r") => {
            if let Some(run) = state.runs.last_mut() {
                run.pieces.push(Piece::Break);
            }
        }
        name if in_properties => {
            let Some(p) = state.runs.last_mut().and_then(|r| r.properties.as_mut()) else {
                return;
            };
            match name {
                "w:rStyle" => {
                    p.style = attr(e, "w:val");
                    p.after_style = Some(span.end);
                }
                "w:rFonts" => p.fonts = Some((span.clone(), legacy_font(e, state.fonts))),
                "w:sz" => p.size = attr(e, "w:val").map(|v| (v, span.end)),
                "w:szCs" => p.complex_size = true,
                _ => {}
            }
        }
        _ => {}
    }
}

fn close(
    name: &str,
    end: usize,
    parent: Option<&str>,
    paragraphs: &mut Vec<Paragraph>,
    runs: &mut Vec<Run>,
    done: &mut Vec<Paragraph>,
) {
    match name {
        "w:rPr" if parent == Some("w:r") => {
            if let Some(p) = runs.last_mut().and_then(|r| r.properties.as_mut()) {
                p.span.end = end;
            }
        }
        "w:r" => {
            if let (Some(run), Some(p)) = (runs.pop(), paragraphs.last_mut()) {
                p.runs.push(run);
            }
        }
        "w:p" => done.extend(paragraphs.pop()),
        _ => {}
    }
}

fn position(reader: &Reader<&[u8]>) -> usize {
    usize::try_from(reader.buffer_position()).unwrap_or(usize::MAX)
}
