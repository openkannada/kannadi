//! Converts the text of 1 DOCX part and the properties of the runs it changes.

use std::ops::Range;

use anyhow::{bail, Result};
use kannadi::akshara::aksharas;
use kannadi::legacy::{syllable_starts, Layout};

use crate::convert::{convert, kannada_runs, Direction, Issue, Settings};
use crate::edits::{apply, overlap};
use crate::report::Row;

use super::scan::{scan, Element, Paragraph, Piece, Properties, Run};
use super::styles::{FontSetting, Styles};
use super::xml;

const BOM: &str = "\u{FEFF}";

/// Converts the text of 1 part. Returns the new part, or `None` if nothing changed.
pub fn convert_part(
    name: &str,
    source: &str,
    styles: &Styles,
    settings: &Settings,
    rows: &mut Vec<Row>,
) -> Result<Option<String>> {
    let (bom, source) = match source.strip_prefix(BOM) {
        Some(rest) => (BOM, rest),
        None => ("", source),
    };
    let to_unicode = matches!(settings.direction, Direction::ToUnicode(_));
    let mut edits = Vec::new();
    let mut fallback_rows = Vec::new();
    for paragraph in &scan(source, &settings.fonts)? {
        let layouts: Vec<Option<Layout>> = paragraph
            .runs
            .iter()
            .map(|r| layout(r, paragraph, styles))
            .collect();
        let location = format!("{name}, paragraph {}", paragraph.number);
        let mut converted = vec![false; paragraph.runs.len()];
        for group in groups(paragraph, &layouts, to_unicode) {
            let edit = Edit {
                source,
                paragraph,
                settings,
                layout: layouts[group[0].0].unwrap_or(settings.text_layout),
                location: &location,
            };
            let fallback = !paragraph.reported || paragraph.runs[group[0].0].fallback;
            let group_rows = if fallback {
                &mut fallback_rows
            } else {
                &mut *rows
            };
            edit.group(&group, &mut edits, group_rows);
            for &(run, _) in &group {
                converted[run] = true;
            }
        }
        if to_unicode {
            let runs = paragraph.runs.iter().zip(&converted).filter(|(_, &c)| c);
            for (run, _) in runs {
                let properties =
                    unicode_properties(source, run.properties.as_ref(), &settings.fonts.unicode);
                match &run.properties {
                    Some(p) => edits.push((p.span.clone(), properties)),
                    None => edits.push((run.content_start..run.content_start, properties)),
                }
            }
        }
    }
    let same =
        |a: &Row, b: &Row| a.kind == b.kind && a.detail == b.detail && a.context == b.context;
    let only_in_fallback: Vec<Row> = fallback_rows
        .into_iter()
        .filter(|f| !rows.iter().any(|r| same(r, f)))
        .collect();
    rows.extend(only_in_fallback);
    if edits.is_empty() {
        return Ok(None);
    }
    if overlap(&edits) {
        bail!("{name}: the part's structure is not one this tool can edit");
    }
    Ok(Some(format!("{bom}{}", apply(source, edits))))
}

/// Runs of consecutive text pieces whose runs are in a legacy font, when `legacy`,
/// or in no legacy font, split at tabs, breaks, runs of another layout or kind,
/// and the edges of tracked changes and alternative content. Each item is (run
/// index, piece index).
fn groups(
    paragraph: &Paragraph,
    layouts: &[Option<Layout>],
    legacy: bool,
) -> Vec<Vec<(usize, usize)>> {
    let mut groups = Vec::new();
    let mut current = Vec::new();
    let mut context = None;
    let mut layout = None;
    for (r, run) in paragraph.runs.iter().enumerate() {
        let wanted = layouts[r].is_some() == legacy;
        if !wanted || run.context != context || layouts[r] != layout {
            groups.push(std::mem::take(&mut current));
            context = run.context;
            layout = layouts[r];
        }
        if !wanted {
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

/// What a group of text pieces is converted against.
struct Edit<'a> {
    source: &'a str,
    paragraph: &'a Paragraph,
    settings: &'a Settings,
    /// The layout the group is read in, or written in.
    layout: Layout,
    location: &'a str,
}

impl Edit<'_> {
    fn group(
        &self,
        group: &[(usize, usize)],
        edits: &mut Vec<(Range<usize>, String)>,
        rows: &mut Vec<Row>,
    ) {
        let pieces: Vec<(&Run, &'static str, &Range<usize>, &Range<usize>)> = group
            .iter()
            .filter_map(|&(r, p)| {
                let run = &self.paragraph.runs[r];
                match &run.pieces[p] {
                    Piece::Text {
                        element,
                        tag,
                        content,
                    } => Some((run, *element, tag, content)),
                    Piece::Break => None,
                }
            })
            .collect();
        let originals: Vec<String> = pieces
            .iter()
            .map(|(_, _, _, c)| xml::decode(&self.source[(*c).clone()]))
            .collect();
        let texts = rebalance(&originals, self.settings.direction, self.layout);
        for ((run, element, tag, content), (text, original)) in
            pieces.iter().zip(texts.iter().zip(&originals))
        {
            let new = match self.settings.direction {
                Direction::ToUnicode(_) => {
                    let converted = convert(text, self.layout, self.settings);
                    add_rows(rows, self.location, &converted.issues);
                    xml::escape(&converted.text)
                }
                Direction::ToNudi => match self.splice(run, element, text, rows) {
                    Some(spliced) => spliced,
                    None if text == original => continue,
                    None => xml::escape(text),
                },
            };
            edits.push((
                (*tag).clone(),
                format!("<{element} xml:space=\"preserve\">"),
            ));
            edits.push(((*content).clone(), new));
        }
    }

    /// The content of a text element with each Kannada run moved into a new run set
    /// in the legacy font, or `None` if `text` has no Kannada.
    fn splice(&self, run: &Run, element: &str, text: &str, rows: &mut Vec<Row>) -> Option<String> {
        let runs = kannada_runs(text);
        if runs.is_empty() {
            return None;
        }
        let own = run
            .properties
            .as_ref()
            .map_or("", |p| &self.source[p.span.clone()]);
        let legacy = legacy_properties(
            self.source,
            run.properties.as_ref(),
            &self.settings.fonts.legacy,
        );
        let open = format!("<{element} xml:space=\"preserve\">");
        let close = format!("</{element}></w:r>");
        let mut out = String::new();
        let mut last = 0;
        for range in runs {
            let converted = convert(&text[range.clone()], self.layout, self.settings);
            add_rows(rows, self.location, &converted.issues);
            out.push_str(&xml::escape(&text[last..range.start]));
            out.push_str(&format!("{close}<w:r>{legacy}{open}"));
            out.push_str(&xml::escape(&converted.text));
            out.push_str(&format!("{close}<w:r>{own}{open}"));
            last = range.end;
        }
        out.push_str(&xml::escape(&text[last..]));
        Some(out)
    }
}

/// Moves the text at each piece boundary that falls inside a syllable to the
/// piece where the syllable starts, so no syllable is split across runs. Legacy
/// text is also never split next to a digit zero, which may be a typed ಂ.
fn rebalance(texts: &[String], direction: Direction, layout: Layout) -> Vec<String> {
    let joined = texts.concat();
    let starts: Vec<usize> = match direction {
        Direction::ToUnicode(_) => syllable_starts(&joined, layout)
            .into_iter()
            .filter(|&s| !joined[..s].ends_with('0') && !joined[s..].starts_with('0'))
            .collect(),
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

fn fonts_element(font: &str) -> String {
    let f = xml::escape(font);
    format!("<w:rFonts w:ascii=\"{f}\" w:hAnsi=\"{f}\" w:eastAsia=\"{f}\" w:cs=\"{f}\"/>")
}

/// The run's `w:rPr` for text converted to Unicode: fonts set to `font`, and the
/// complex-script copies of bold, italic and size added where only the Latin
/// ones are set, since Word formats Kannada with `w:bCs`, `w:iCs` and `w:szCs`.
fn unicode_properties(source: &str, properties: Option<&Properties>, font: &str) -> String {
    let fonts = fonts_element(font);
    let Some(p) = properties.filter(|p| !p.empty) else {
        return format!("<w:rPr>{fonts}</w:rPr>");
    };
    let mut edits = vec![fonts_edit(p, fonts)];
    for (latin, complex, name) in [
        (&p.bold, &p.bold_cs, "w:bCs"),
        (&p.italic, &p.italic_cs, "w:iCs"),
        (&p.size, &p.size_cs, "w:szCs"),
    ] {
        if let (Some(latin), None) = (latin, complex) {
            let at = latin.span.end;
            edits.push((at..at, format!("<{name}{}/>", attributes(latin))));
        }
    }
    rewrite(source, p, edits)
}

/// The run's `w:rPr` for text converted to Nudi: fonts set to `font`, and the
/// Latin copies of bold, italic and size taken from the complex-script ones,
/// since Word formats the legacy glyph codes with `w:b`, `w:i` and `w:sz`.
fn legacy_properties(source: &str, properties: Option<&Properties>, font: &str) -> String {
    let fonts = fonts_element(font);
    let Some(p) = properties.filter(|p| !p.empty) else {
        return format!("<w:rPr>{fonts}</w:rPr>");
    };
    let mut edits = vec![fonts_edit(p, fonts)];
    for (latin, complex, name) in [
        (&p.bold, &p.bold_cs, "w:b"),
        (&p.italic, &p.italic_cs, "w:i"),
        (&p.size, &p.size_cs, "w:sz"),
    ] {
        let Some(complex) = complex else { continue };
        let element = format!("<{name}{}/>", attributes(complex));
        match latin {
            Some(latin) => edits.push((latin.span.clone(), element)),
            None => {
                let at = complex.span.start;
                edits.push((at..at, element));
            }
        }
    }
    rewrite(source, p, edits)
}

/// The edit that sets the fonts: in place of `w:rFonts`, or else after
/// `w:rStyle`, which must come first.
fn fonts_edit(p: &Properties, fonts: String) -> (Range<usize>, String) {
    match (&p.fonts, &p.style_element) {
        (Some(f), _) => (f.span.clone(), fonts),
        (None, Some(style)) => (style.span.end..style.span.end, fonts),
        (None, None) => (p.content_start..p.content_start, fonts),
    }
}

/// An element's attributes, without the slash of an empty-element tag.
fn attributes(element: &Element) -> &str {
    element.attributes.trim_end().trim_end_matches('/')
}

/// The `w:rPr` with `edits`, given in positions in `source`.
fn rewrite(source: &str, p: &Properties, edits: Vec<(Range<usize>, String)>) -> String {
    let base = p.span.start;
    let edits = edits
        .into_iter()
        .map(|(r, text)| (r.start - base..r.end - base, text))
        .collect();
    apply(&source[p.span.clone()], edits)
}

/// The layout of a run set in a legacy font: by its own fonts, then its character
/// style, then its paragraph style or the default paragraph style, then the
/// document defaults.
fn layout(run: &Run, paragraph: &Paragraph, styles: &Styles) -> Option<Layout> {
    let p = run.properties.as_ref();
    let character = || {
        p.and_then(|p| p.style.as_deref())
            .map_or(FontSetting::Inherit, |s| styles.font(s))
    };
    p.map_or(FontSetting::Inherit, |p| p.font)
        .or_else(character)
        .or_else(|| styles.paragraph(paragraph.style.as_deref()))
        .or_else(|| styles.document_default())
        .layout()
}

fn add_rows(rows: &mut Vec<Row>, location: &str, issues: &[Issue]) {
    rows.extend(
        issues
            .iter()
            .map(|issue| Row::new(location.to_string(), issue)),
    );
}
