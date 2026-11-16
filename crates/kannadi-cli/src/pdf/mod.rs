//! PDF files, read for their text and written as UTF-8 text. Text in legacy fonts
//! is converted; other text is kept. A legacy font is found by its name, or by its
//! text: mostly Latin-1 glyph codes that convert into clean Kannada syllables. A
//! font named as legacy whose codes were renumbered is read through its ToUnicode
//! map when that gives the legacy codes back. Fonts that cannot be read are
//! reported and left out.

mod cmap;
mod encoding;
mod fonts;
mod reader;
mod widths;

use anyhow::{bail, Context, Result};
use kannadi::legacy::{syllable_starts, to_unicode, Layout, Options};
use lopdf::{Document, Object};

use crate::convert::{convert, Direction, Issue, Kind, Settings};
use crate::report::{Report, Row};
use fonts::{Fonts, Source, Verdict};
use reader::{read_page, Piece};

/// The most a single page or font stream may decompress to.
const LIMIT: usize = 64 << 20;

/// The object a reference points to, or the object itself.
fn resolve<'a>(doc: &'a Document, object: &'a Object) -> Option<&'a Object> {
    match object {
        Object::Reference(id) => doc.get_object(*id).ok(),
        other => Some(other),
    }
}

pub fn convert_file(bytes: &[u8], settings: &Settings) -> Result<(Vec<u8>, Report)> {
    convert_with_limit(bytes, settings, LIMIT)
}

fn convert_with_limit(
    bytes: &[u8],
    settings: &Settings,
    page_limit: usize,
) -> Result<(Vec<u8>, Report)> {
    let Direction::ToUnicode(options) = settings.direction else {
        bail!("PDF files can only be converted to Unicode");
    };
    let doc = Document::load_mem(bytes).context("not a readable PDF")?;
    let mut fonts = Fonts::default();
    let mut report = Report::default();
    let mut pages = Vec::new();
    for (number, id) in doc.get_pages() {
        match read_page(&doc, id, &mut fonts, settings, page_limit) {
            Ok(page) => {
                if page.lost > 0 {
                    report.rows.push(not_converted(
                        format!("page {number}"),
                        format!("{} strings in a font the page does not define", page.lost),
                    ));
                }
                pages.push((number, page.lines));
            }
            Err(e) => {
                report
                    .rows
                    .push(not_converted(format!("page {number}"), format!("{e:#}")));
                pages.push((number, Vec::new()));
            }
        }
    }
    fonts.decide();
    for (font, reason) in fonts.skipped() {
        report
            .rows
            .push(not_converted(format!("font {font}"), reason.to_string()));
    }

    let writer = LineWriter::new(&fonts, options, settings);
    let mut out = String::new();
    for (i, (number, lines)) in pages.iter().enumerate() {
        if i > 0 {
            out.push('\u{0C}');
        }
        for (l, line) in lines.iter().enumerate() {
            let location = format!("page {number}, line {}", l + 1);
            out.push_str(&writer.write(line, &location, &mut report));
            out.push('\n');
        }
    }
    Ok((out.into_bytes(), report))
}

fn not_converted(location: String, detail: String) -> Row {
    Row {
        location,
        kind: Kind::NotConverted,
        detail,
        context: String::new(),
    }
}

/// Writes lines: runs of legacy text are joined and converted together.
struct LineWriter<'a> {
    fonts: &'a Fonts,
    /// For text in the `e` fonts and for text in the `k` fonts, which draw Kannada
    /// digits.
    latin_digits: Settings,
    kannada_digits: Settings,
}

impl<'a> LineWriter<'a> {
    fn new(fonts: &'a Fonts, options: Options, settings: &Settings) -> Self {
        let with = |kannada_digits: bool| Settings {
            direction: Direction::ToUnicode(Options {
                kannada_digits: options.kannada_digits || kannada_digits,
                ..options
            }),
            ..settings.clone()
        };
        LineWriter {
            fonts,
            latin_digits: with(false),
            kannada_digits: with(true),
        }
    }

    fn write(&self, line: &[Piece], location: &str, report: &mut Report) -> String {
        let mut out = String::new();
        let mut legacy = String::new();
        let mut reading = (Layout::Nudi, false);
        let flush = |legacy: &mut String,
                     (layout, digits): (Layout, bool),
                     out: &mut String,
                     report: &mut Report| {
            if legacy.is_empty() {
                return;
            }
            let settings = if digits {
                &self.kannada_digits
            } else {
                &self.latin_digits
            };
            let converted = convert(legacy, layout, settings);
            add_rows(report, location, &converted.issues);
            out.push_str(&converted.text);
            legacy.clear();
        };
        let mut gap = false;
        let space = |gap: &mut bool, legacy: &mut String, out: &mut String| {
            if std::mem::take(gap) {
                if legacy.is_empty() {
                    out.push(' ');
                } else {
                    legacy.push(' ');
                }
            }
        };
        for piece in line {
            let Piece::Text { font, raw, mapped } = piece else {
                gap = true;
                continue;
            };
            let font = &self.fonts.fonts[*font];
            match font.verdict {
                Verdict::Legacy(source) => {
                    let text = match (source, mapped) {
                        (Source::Mapped, Some(mapped)) => mapped,
                        _ => raw,
                    };
                    if text.is_empty() {
                        continue;
                    }
                    let font_reading = (font.layout(), font.kannada_digits);
                    if font_reading == reading && gap && inside_syllable(&legacy, text, reading.0) {
                        gap = false;
                    }
                    if font_reading != reading {
                        space(&mut gap, &mut legacy, &mut out);
                        flush(&mut legacy, reading, &mut out, report);
                        reading = font_reading;
                    }
                    space(&mut gap, &mut legacy, &mut out);
                    legacy.push_str(text);
                }
                Verdict::Unreadable(_) => {
                    space(&mut gap, &mut legacy, &mut out);
                    flush(&mut legacy, reading, &mut out, report);
                }
                Verdict::Other if raw.is_empty() => {}
                Verdict::Other => {
                    space(&mut gap, &mut legacy, &mut out);
                    flush(&mut legacy, reading, &mut out, report);
                    out.push_str(raw);
                }
            }
        }
        space(&mut gap, &mut legacy, &mut out);
        flush(&mut legacy, reading, &mut out, report);
        out.trim_end().to_string()
    }
}

/// Whether a gap the page leaves between `legacy` and `next` falls inside a
/// syllable: a space there would leave more problems than none, as before a sign
/// or subscript. Only the last 2 syllables of `legacy` are converted again, since
/// a letter drawn with several glyphs, such as Nudi's ಯ, may read as 2 when cut.
fn inside_syllable(legacy: &str, next: &str, layout: Layout) -> bool {
    let window = legacy.char_indices().rev().nth(31).map_or(0, |(i, _)| i);
    let recent = &legacy[window..];
    let starts = syllable_starts(recent, layout);
    let Some(&from) = starts.iter().rev().nth(1).or(starts.first()) else {
        return false;
    };
    let tail = &recent[from..];
    let problems = |text: &str| {
        let c = to_unicode(text, layout, &Options::default());
        c.unknown.len() + c.malformed.len()
    };
    problems(&format!("{tail} {next}")) > problems(&format!("{tail}{next}"))
}

fn add_rows(report: &mut Report, location: &str, issues: &[Issue]) {
    report
        .rows
        .extend(issues.iter().map(|i| Row::new(location.to_string(), i)));
}

#[cfg(test)]
mod tests {
    use super::fonts::{NO_TABLE, OTHER_LAYOUT, RENUMBERED};
    use super::*;
    use kannadi::legacy::Options;
    use lopdf::{dictionary, Dictionary, Object, Stream};

    /// A 1-page PDF with the given fonts and content stream.
    fn pdf(fonts: Vec<(&str, Dictionary)>, content: &[u8]) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let pages = doc.new_object_id();
        let mut font_refs = Dictionary::new();
        for (name, font) in fonts {
            let id = doc.add_object(font);
            font_refs.set(name, Object::Reference(id));
        }
        let contents = doc.add_object(Stream::new(dictionary! {}, content.to_vec()));
        let page = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages,
            "Contents" => contents,
            "Resources" => dictionary! { "Font" => font_refs },
        });
        doc.objects.insert(
            pages,
            Object::Dictionary(
                dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 },
            ),
        );
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", catalog);
        let mut out = Vec::new();
        doc.save_to(&mut out).unwrap();
        out
    }

    fn simple_font(name: &str) -> Dictionary {
        dictionary! {
            "Type" => "Font",
            "Subtype" => "TrueType",
            "BaseFont" => name,
            "Encoding" => "WinAnsiEncoding",
            "FirstChar" => 32,
            "Widths" => vec![Object::Integer(500); 224],
        }
    }

    fn run(bytes: &[u8]) -> (String, Report) {
        let settings = Settings::new(Direction::ToUnicode(Options::default()));
        let (out, report) = convert_file(bytes, &settings).unwrap();
        (String::from_utf8(out).unwrap(), report)
    }

    const KANNADA: &[u8] = b"P\xC0\xA3\xC0\xDFq\xC0";

    #[test]
    fn named_legacy_fonts_convert_and_others_stay() {
        let mut content = b"BT /F1 12 Tf 72 700 Td (".to_vec();
        content.extend_from_slice(KANNADA);
        content.extend_from_slice(b") Tj /F2 12 Tf 60 0 Td (Office) Tj ET");
        let fonts = vec![
            ("F1", simple_font("ABCDEF+BRHKannada")),
            ("F2", simple_font("Arial")),
        ];
        let (text, report) = run(&pdf(fonts, &content));
        assert_eq!(text, "ಕನ್ನಡ Office\n");
        assert!(report.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn named_legacy_fonts_without_an_encoding_give_raw_codes() {
        let mut font = simple_font("BRHKannada");
        font.remove(b"Encoding");
        let mut content = b"BT /F1 12 Tf 72 700 Td (".to_vec();
        content.extend_from_slice(KANNADA);
        content.extend_from_slice(b") Tj ET");
        let (text, _) = run(&pdf(vec![("F1", font)], &content));
        assert_eq!(text, "ಕನ್ನಡ\n");
    }

    #[test]
    fn fragments_join_and_wide_gaps_are_spaces() {
        // Each glyph is 6 units wide at 12 points: "P\xC0" ends at 12.
        let content =
            b"BT /F1 12 Tf 72 700 Td (P\xC0) Tj 12 0 Td (\xA3\xC0\xDF) Tj 18 0 Td (q\xC0) Tj \
                        30 0 Td (P\xC0) Tj 0 -14 Td (q\xC0) Tj ET";
        let (text, _) = run(&pdf(vec![("F1", simple_font("BRHKannada"))], content));
        assert_eq!(text, "ಕನ್ನಡ ಕ\nಡ\n");
    }

    #[test]
    fn a_font_with_a_hidden_name_is_found_by_its_text() {
        let (doc, font) = cid_font(None);
        let shown = "<0001000200030002000400010002>".repeat(6);
        let content = format!("BT /F1 12 Tf 72 700 Td {shown} Tj ET");
        let (text, report) = run(&pdf_with_objects(doc, vec![font], content.as_bytes()));
        assert!(text.starts_with("ಕನ ಕಕನ ಕ"), "{text:?}");
        assert!(report.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn cid_widths_place_word_gaps() {
        // With W, the 8 glyphs of a word take 24 units at 12 points; with DW alone, 48.
        let (doc, font) = cid_font(Some(vec![1.into(), vec![Object::Integer(250); 4].into()]));
        let word = "<00010002000300020001000200030002>";
        let shown = format!("{word} Tj 30 0 Td ").repeat(3);
        let content = format!("BT /F1 12 Tf 72 700 Td {shown} ET");
        let (text, _) = run(&pdf_with_objects(doc, vec![font], content.as_bytes()));
        assert!(text.starts_with("ಕನಕನ ಕನಕನ"), "{text:?}");
    }

    /// A CID font with a hidden name whose ToUnicode map gives Nudi codes for
    /// CIDs 1 to 4: P, À, £ and a space.
    fn cid_font(widths: Option<Vec<Object>>) -> (Document, Dictionary) {
        let cmap = b"/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n\
/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n\
/CMapName /Adobe-Identity-UCS def\n/CMapType 2 def\n\
1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange\n\
4 beginbfchar\n<0001> <0050>\n<0002> <00C0>\n<0003> <00A3>\n<0004> <0020>\nendbfchar\n\
endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend\n";
        let mut descendant = dictionary! {
            "Type" => "Font", "Subtype" => "CIDFontType2", "BaseFont" => "CIDFont+F1", "DW" => 500,
        };
        if let Some(w) = widths {
            descendant.set("W", w);
        }
        let mut doc = Document::with_version("1.5");
        let d = doc.add_object(descendant);
        let t = doc.add_object(Stream::new(dictionary! {}, cmap.to_vec()));
        let font = dictionary! {
            "Type" => "Font", "Subtype" => "Type0", "BaseFont" => "CIDFont+F1",
            "Encoding" => "Identity-H", "DescendantFonts" => vec![d.into()], "ToUnicode" => t,
        };
        (doc, font)
    }

    #[test]
    fn text_in_forms_is_read_where_the_forms_place_it() {
        let mut doc = Document::with_version("1.5");
        let font = doc.add_object(simple_font("BRHKannada"));
        let mut form_text = b"BT /K 12 Tf 72 700 Td (".to_vec();
        form_text.extend_from_slice(KANNADA);
        form_text.extend_from_slice(b") Tj ET");
        let mut form = |matrix: Vec<Object>| {
            doc.add_object(Stream::new(
                dictionary! {
                    "Type" => "XObject", "Subtype" => "Form", "Matrix" => matrix,
                    "Resources" => dictionary! { "Font" => dictionary! { "K" => font } },
                },
                form_text.clone(),
            ))
        };
        let identity = form(vec![
            1.into(),
            0.into(),
            0.into(),
            1.into(),
            0.into(),
            0.into(),
        ]);
        let lower = form(vec![
            1.into(),
            0.into(),
            0.into(),
            1.into(),
            0.into(),
            (-20).into(),
        ]);
        let pages = doc.new_object_id();
        let contents = doc.add_object(Stream::new(
            dictionary! {},
            b"/X1 Do /X1 Do /X2 Do /X1 Do".to_vec(),
        ));
        let page = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages, "Contents" => contents,
            "Resources" => dictionary! { "XObject" => dictionary! { "X1" => identity, "X2" => lower } },
        });
        doc.objects.insert(
            pages,
            Object::Dictionary(
                dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 },
            ),
        );
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", catalog);
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        let (text, _) = run(&bytes);
        assert_eq!(text, "ಕನ್ನಡಕನ್ನಡ\nಕನ್ನಡ\nಕನ್ನಡ\n");
    }

    #[test]
    fn small_vertical_moves_and_tj_gaps() {
        let content =
            b"BT /F1 12 Tf 72 700 Td (P) Tj 6 -2 Td (\xC0) Tj [(P\xC0) -300 (P\xC0)] TJ ET";
        let (text, report) = run(&pdf(vec![("F1", simple_font("BRHKannada"))], content));
        assert_eq!(text, "ಕಕ ಕ\n");
        assert!(report.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn ascii_text_in_other_fonts_is_never_converted() {
        let content = b"BT /F1 12 Tf 72 700 Td (Qa Qa Qa Qa Qa Qa Qa) Tj ET";
        let (text, _) = run(&pdf(vec![("F1", simple_font("Arial"))], content));
        assert_eq!(text, "Qa Qa Qa Qa Qa Qa Qa\n");
    }

    #[test]
    fn a_named_font_whose_text_does_not_convert_is_reported() {
        let content =
            b"BT /F1 12 Tf 72 700 Td (The quick brown fox jumps over the lazy dog again) Tj ET";
        let (text, report) = run(&pdf(vec![("F1", simple_font("BRH Kannada"))], content));
        assert_eq!(report.rows[0].detail, RENUMBERED);
        assert_eq!(text, "\n");
    }

    #[test]
    fn a_renumbered_named_font_is_read_through_its_map() {
        // Codes from ! up, mapped back to the Nudi codes of ಕನ್ನಡ by ToUnicode.
        let cmap = b"/CMapName //ABCDEF+Nudi01k def\nbegincmap\n\
            5 beginbfchar\n<21> <0050>\n<22> <00C0>\n<23> <00A3>\n<24> <00DF>\n<25> <0071>\nendbfchar\nendcmap\n";
        for words in [4, 1] {
            let mut doc = Document::with_version("1.5");
            let map = doc.add_object(Stream::new(dictionary! {}, cmap.to_vec()));
            let mut font = simple_font("ABCDEF+Nudi01k");
            font.set("ToUnicode", map);
            let shown = "!\"#\"$%\" ".repeat(words);
            let content = format!("BT /F1 12 Tf 72 700 Td ({shown}) Tj ET");
            let (text, report) = run(&pdf_with_objects(doc, vec![font], content.as_bytes()));
            assert_eq!(text, format!("{}\n", "ಕನ್ನಡ ".repeat(words).trim_end()));
            assert!(report.is_empty(), "{:?}", report.rows);
        }
    }

    #[test]
    fn short_text_that_does_not_convert_cleanly_is_kept() {
        for (shown, kept) in [
            (
                &b"\x95 \x95 \x95 \x95"[..],
                "\u{2022} \u{2022} \u{2022} \u{2022}",
            ),
            (b"\xC1\xC1 \xC1\xC1", "\u{C1}\u{C1} \u{C1}\u{C1}"),
        ] {
            let mut content = b"BT /F1 12 Tf 72 700 Td (".to_vec();
            content.extend_from_slice(shown);
            content.extend_from_slice(b") Tj ET");
            let (text, report) = run(&pdf(vec![("F1", simple_font("ABCDEF+F1"))], &content));
            assert_eq!(text, format!("{kept}\n"));
            assert!(report.is_empty(), "{:?}", report.rows);
        }
    }

    #[test]
    fn glyph_names_are_read_on_the_right_base() {
        let with_base = |base: Option<&str>| {
            let mut encoding = dictionary! {
                "Differences" => vec![Object::Integer(65), Object::Name(b"fi".to_vec())],
            };
            if let Some(base) = base {
                encoding.set("BaseEncoding", base);
            }
            let mut font = simple_font("Arial");
            font.set("Encoding", encoding);
            font
        };
        let content = b"BT /F1 12 Tf 72 700 Td (A\x8E) Tj ET";
        let (text, _) = run(&pdf(vec![("F1", with_base(None))], content));
        assert_eq!(text, "\u{FB01}\u{17D}\n");
        let (text, _) = run(&pdf(
            vec![("F1", with_base(Some("MacRomanEncoding")))],
            content,
        ));
        assert_eq!(text, "\u{FB01}\u{E9}\n");
    }

    #[test]
    fn fonts_with_a_differences_list_are_read_by_code() {
        let mut font = simple_font("SXVTNG+TTE10AECA0t00");
        let differences = vec![Object::Integer(1), Object::Name(b"c129".to_vec())];
        font.set(
            "Encoding",
            dictionary! { "BaseEncoding" => Object::Name(b"WinAnsiEncoding".to_vec()), "Differences" => differences },
        );
        let content = b"BT /F1 12 Tf 72 700 Td (\xA8s\xC1U\xC0 \xA8s\xC1U\xC0) Tj ET";
        let (text, report) = run(&pdf(vec![("F1", font)], content));
        assert_eq!(text, "ಭಾಗ ಭಾಗ\n");
        assert!(report.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn each_text_object_starts_from_the_page_origin() {
        let content = b"BT /F1 12 Tf 72 700 Td (P\xC0) Tj ET BT /F1 12 Tf 90 700 Td (P\xC0) Tj ET";
        let (text, _) = run(&pdf(vec![("F1", simple_font("BRHKannada"))], content));
        assert_eq!(text, "ಕಕ\n");
    }

    #[test]
    fn text_placed_by_the_transformation_matrix_keeps_its_lines() {
        let content = b"q 1 0 0 1 72 700 cm BT /F1 12 Tf (P\xC0) Tj ET Q \
            q 1 0 0 1 72 680 cm BT /F1 12 Tf (P\xC0) Tj ET Q \
            q 1 0 0 1 72 680 cm 2 0 0 2 0 0 cm BT /F1 6 Tf 9 0 Td (P\xC0) Tj ET Q";
        let (text, _) = run(&pdf(vec![("F1", simple_font("BRHKannada"))], content));
        assert_eq!(text, "ಕ\nಕಕ\n");
    }

    #[test]
    fn line_breaks_are_measured_on_the_page() {
        let content = b"0.05 0 0 0.05 0 0 cm BT /F1 240 Tf 1440 14000 Td (P\xC0) Tj \
            0 -300 Td (P\xC0) Tj ET";
        let (text, _) = run(&pdf(vec![("F1", simple_font("BRHKannada"))], content));
        assert_eq!(text, "ಕ\nಕ\n");
    }

    #[test]
    fn short_legacy_text_in_an_unknown_font_is_converted() {
        let content = b"BT /F1 12 Tf 72 700 Td (\xA8s\xC1U\xC0) Tj ET";
        let (text, _) = run(&pdf(vec![("F1", simple_font("CIDFont+F1"))], content));
        assert_eq!(text, "ಭಾಗ\n");
    }

    #[test]
    fn strings_without_a_font_are_reported() {
        let content = b"BT 72 700 Td (lost) Tj /Missing 12 Tf (also lost) Tj ET";
        let (text, report) = run(&pdf(vec![("F1", simple_font("Arial"))], content));
        assert_eq!(text, "\n");
        assert_eq!(report.rows[0].location, "page 1");
        assert_eq!(report.rows[0].kind, Kind::NotConverted);
    }

    /// A page with fonts F1 and F2 that draws form `/X`, whose own font F3 is
    /// Arial.
    fn page_with_form(page: &[u8], form: &[u8], form_references_itself: bool) -> Vec<u8> {
        let mut doc = Document::with_version("1.5");
        let nudi = doc.add_object(simple_font("BRHKannada"));
        let arial = doc.add_object(simple_font("Arial"));
        let form_id = doc.new_object_id();
        let mut form_resources = dictionary! { "Font" => dictionary! { "F3" => arial } };
        if form_references_itself {
            form_resources.set("XObject", dictionary! { "X" => form_id });
        }
        doc.objects.insert(
            form_id,
            Object::Stream(Stream::new(
                dictionary! { "Type" => "XObject", "Subtype" => "Form", "Resources" => form_resources },
                form.to_vec(),
            )),
        );
        let pages = doc.new_object_id();
        let contents = doc.add_object(Stream::new(dictionary! {}, page.to_vec()));
        let page = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages, "Contents" => contents,
            "Resources" => dictionary! {
                "Font" => dictionary! { "F1" => nudi, "F2" => arial },
                "XObject" => dictionary! { "X" => form_id },
            },
        });
        doc.objects.insert(
            pages,
            Object::Dictionary(
                dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 },
            ),
        );
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", catalog);
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        bytes
    }

    #[test]
    fn a_form_keeps_its_font_to_itself() {
        let page = b"BT /F1 12 Tf ET /X Do BT 72 600 Td (P\xC0) Tj ET";
        let form = b"BT /F3 12 Tf 72 650 Td (x) Tj ET";
        let (text, _) = run(&page_with_form(page, form, false));
        assert_eq!(text, "x\nಕ\n");
    }

    #[test]
    fn a_form_cannot_restore_the_state_the_page_saved() {
        let page = b"BT /F1 12 Tf ET q BT /F2 12 Tf ET /X Do Q BT 72 600 Td (P\xC0) Tj ET";
        let (text, _) = run(&page_with_form(page, b"Q", false));
        assert_eq!(text, "ಕ\n");
    }

    #[test]
    fn forms_that_draw_each_other_many_times_stop() {
        let form = b"/X Do ".repeat(20);
        let (_, report) = run(&page_with_form(b"/X Do", &form, true));
        assert_eq!(report.rows[0].location, "page 1");
        assert_eq!(report.rows[0].kind, Kind::NotConverted);
    }

    #[test]
    fn a_page_that_cannot_be_read_is_reported_and_the_rest_converted() {
        let mut doc = Document::with_version("1.5");
        let font = doc.add_object(simple_font("BRHKannada"));
        let pages = doc.new_object_id();
        let broken = doc.add_object(Stream::new(dictionary! {}, vec![b' '; 100]));
        let fine = doc.add_object(Stream::new(
            dictionary! {},
            b"BT /F1 12 Tf 72 700 Td (P\xC0) Tj ET".to_vec(),
        ));
        let kids: Vec<Object> = [broken, fine]
            .into_iter()
            .map(|contents| {
                doc.add_object(dictionary! {
                    "Type" => "Page", "Parent" => pages, "Contents" => contents,
                    "Resources" => dictionary! { "Font" => dictionary! { "F1" => font } },
                })
                .into()
            })
            .collect();
        doc.objects.insert(
            pages,
            Object::Dictionary(dictionary! { "Type" => "Pages", "Kids" => kids, "Count" => 2 }),
        );
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", catalog);
        let mut bytes = Vec::new();
        doc.save_to(&mut bytes).unwrap();
        let settings = Settings::new(Direction::ToUnicode(Options::default()));
        let (text, report) = convert_with_limit(&bytes, &settings, 50).unwrap();
        assert_eq!(String::from_utf8(text).unwrap(), "\u{c}ಕ\n");
        assert_eq!(report.rows.len(), 1, "{:?}", report.rows);
        assert_eq!(report.rows[0].location, "page 1");
        assert_eq!(report.rows[0].kind, Kind::NotConverted);
    }

    /// Like `pdf`, for fonts whose parts were added to `doc` already.
    fn pdf_with_objects(mut doc: Document, fonts: Vec<Dictionary>, content: &[u8]) -> Vec<u8> {
        let pages = doc.new_object_id();
        let mut font_refs = Dictionary::new();
        for (i, font) in fonts.into_iter().enumerate() {
            let id = doc.add_object(font);
            font_refs.set(format!("F{}", i + 1), Object::Reference(id));
        }
        let contents = doc.add_object(Stream::new(dictionary! {}, content.to_vec()));
        let page = doc.add_object(dictionary! {
            "Type" => "Page", "Parent" => pages, "Contents" => contents,
            "Resources" => dictionary! { "Font" => font_refs },
        });
        doc.objects.insert(
            pages,
            Object::Dictionary(
                dictionary! { "Type" => "Pages", "Kids" => vec![page.into()], "Count" => 1 },
            ),
        );
        let catalog = doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages });
        doc.trailer.set("Root", catalog);
        let mut out = Vec::new();
        doc.save_to(&mut out).unwrap();
        out
    }

    #[test]
    fn unreadable_fonts_are_reported_and_left_out() {
        let renumbered =
            b"BT /F1 12 Tf 72 700 Td (!\"#$%&'()*+,-./!\"#$%&'()*+,-./ABCDEFGHIJ) Tj ET";
        let mut font = simple_font("Nudi01k");
        let mut differences = vec![Object::Integer(33)];
        differences.extend((1..=10).map(|i| Object::Name(format!("g{i}").into_bytes())));
        font.set(
            "Encoding",
            dictionary! { "Type" => "Encoding", "Differences" => differences },
        );
        let (text, report) = run(&pdf(vec![("F1", font)], renumbered));
        assert_eq!(text, "\n");
        assert_eq!(report.rows[0].kind, Kind::NotConverted);
        assert_eq!(report.rows[0].location, "font Nudi01k");
        let (_, report) = run(&pdf(vec![("F1", simple_font("F1"))], AKLITE));
        assert_eq!(report.rows[0].detail, OTHER_LAYOUT);
    }

    /// AkliteKndPadmini text from the Repealing and Amending Act, 2003, twice,
    /// which fits no layout this tool knows.
    const AKLITE: &[u8] = b"BT /F1 12 Tf 72 700 Td (\xB0\xF0\xC2\xF1 \xF0 \x8B\xC7\xF0\xCD\xF0\xC2\xF0 \xB3\xFA\xF7\x92\xCD\xF0\xF4\xC6\xF0 \xC6\xF0\xF4\xC0\xF0\xF4\xDE \x89\xC1\xF0\xF4\xE0\xC3\xF0\x88 \xB0\xF0\xC2\xF1 \xF0 \x8B\xC7\xF0\xCD\xF0\xC2\xF0 \xB3\xFA\xF7\x92\xCD\xF0\xF4\xC6\xF0 \xC6\xF0\xF4\xC0\xF0\xF4\xDE \x89\xC1\xF0\xF4\xE0\xC3\xF0\x88) Tj ET";

    /// ರೋರಿಕ್ ಮತ್ತು ದೇವಿಕಾರಾಣಿ ರೋರಿಕ್, from the title of the Roerich Act, 2001.
    const AKRUTI: &[u8] = b"p\xEA\xB3\xA9\xEEd\xD2 \xC0\xE2\xB0y\xE2\xB0K {\xEA\xA9\xC6d\xD6p\xD6\xB9 p\xEA\xB3\xA9\xEEd\xD2";

    fn shown(font: &str, bytes: &[u8]) -> Vec<u8> {
        let mut content = format!("BT /{font} 12 Tf 72 700 Td (").into_bytes();
        content.extend_from_slice(bytes);
        content.extend_from_slice(b") Tj ET");
        content
    }

    #[test]
    fn a_legacy_font_with_no_table_is_reported_and_left_out() {
        let (text, report) = run(&pdf(
            vec![("F1", simple_font("ABCDEF+Shree-Kan-0850"))],
            &shown("F1", b"\xC3\xDC\xE3\xB1\xDC\xF1\xDD\xC3\xDD"),
        ));
        assert_eq!(text, "\n");
        assert_eq!(report.rows[0].detail, NO_TABLE);
    }

    #[test]
    fn digits_in_a_legacy_font_with_no_table_stay() {
        let (text, report) = run(&pdf(
            vec![("F1", simple_font("Shree-Kan-0850"))],
            &shown("F1", b"2002-03"),
        ));
        assert_eq!(text, "2002-03\n");
        assert!(report.rows.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn an_added_name_gives_a_no_table_font_its_layout() {
        let mut settings = Settings::new(Direction::ToUnicode(Options::default()));
        settings.fonts.extra_legacy = vec![("Shree-Kan-0850".to_string(), Layout::Nudi)];
        let bytes = pdf(
            vec![("F1", simple_font("Shree-Kan-0850"))],
            &shown("F1", KANNADA),
        );
        let (out, report) = convert_file(&bytes, &settings).unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "ಕನ್ನಡ\n");
        assert!(report.rows.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn akruti_fonts_are_read_in_their_layout() {
        let (text, report) = run(&pdf(
            vec![("F1", simple_font("AkrutiKndPadma"))],
            &shown("F1", AKRUTI),
        ));
        assert_eq!(text, "ರೋರಿಕ್ ಮತ್ತು ದೇವಿಕಾರಾಣಿ ರೋರಿಕ್\n");
        assert!(report.rows.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn short_text_in_a_font_with_a_hidden_name_is_read_in_its_best_layout() {
        // ಇತರ ಕ್ರಮ reads with no problems as Nudi too, as clusters.
        let (text, _) = run(&pdf(
            vec![("F1", simple_font("F1"))],
            &shown("F1", b"Ey\xE2p\xE2 d\xE2Z\xC0\xE2\xB0"),
        ));
        assert_eq!(text, "ಇತರ ಕ್ರಮ\n");
    }

    #[test]
    fn fonts_with_hidden_names_are_read_in_the_layout_their_text_fits() {
        let (text, _) = run(&pdf(vec![("F1", simple_font("F1"))], &shown("F1", AKRUTI)));
        assert_eq!(text, "ರೋರಿಕ್ ಮತ್ತು ದೇವಿಕಾರಾಣಿ ರೋರಿಕ್\n");
    }

    #[test]
    fn glyphs_named_by_their_code_are_read_as_that_code() {
        // Distiller renumbers the codes from 1 and names each glyph G<code>.
        let mut font = simple_font("BEEKHI+AkrutiKndPadmaDemi0108.313");
        let names = ["G100", "G226", "G125", "G32"];
        let mut differences = vec![Object::Integer(1)];
        differences.extend(names.iter().map(|n| Object::Name(n.as_bytes().to_vec())));
        font.set(
            "Encoding",
            dictionary! { "Type" => "Encoding", "Differences" => differences },
        );
        let (text, _) = run(&pdf(
            vec![("F1", font)],
            &shown("F1", b"\x01\x02\x03\x02\x04\x01\x02"),
        ));
        assert_eq!(text, "ಕನ ಕ\n");
    }

    #[test]
    fn a_gap_before_a_sign_is_not_a_word_space() {
        let content = b"BT /F1 12 Tf 72 700 Td (d) Tj 30 0 Td (\xE2}\xE2) Tj 30 0 Td (d\xE2) Tj ET";
        let (text, report) = run(&pdf(vec![("F1", simple_font("AkrutiKndPadma"))], content));
        assert_eq!(text, "ಕನ ಕ\n");
        assert!(report.rows.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn a_gap_inside_a_letter_drawn_with_several_glyphs_is_not_a_word_space() {
        // ಗೆಯು, with a gap inside the 4 glyphs of ಯ.
        let content = b"BT /F1 12 Tf 72 700 Td (U\xC9Ai) Tj 30 0 Td (\xC0\xC4\xC4) Tj ET";
        let (text, report) = run(&pdf(vec![("F1", simple_font("BRH Kannada"))], content));
        assert_eq!(text, "ಗೆಯು\n");
        assert!(report.rows.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn a_gap_before_a_spacing_glyph_is_a_word_space() {
        let content = b"BT /F1 12 Tf 72 700 Td (d\xE2) Tj 30 0 Td (<}\xE2) Tj ET";
        let (text, _) = run(&pdf(vec![("F1", simple_font("AkrutiKndPadma"))], content));
        assert_eq!(text, "ಕ ನ\n");
    }

    #[test]
    fn several_gaps_and_empty_strings_make_1_gap() {
        let content =
            b"BT /F1 12 Tf 72 700 Td [(d) -300 () -300 (\xE2}\xE2) -300 -300 (d\xE2)] TJ ET";
        let (text, report) = run(&pdf(vec![("F1", simple_font("AkrutiKndPadma"))], content));
        assert_eq!(text, "ಕನ ಕ\n");
        assert!(report.rows.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn short_text_in_a_named_font_reads_though_it_has_symbol_codes() {
        // ದಿನಾಂಕ ಪತ್ರ ದಾನ: Akruti draws ದ, ನ and ಪ at { } ~.
        let short = b"\xAC}\xD6\xAAd\xE2 ~\xE2y\xE2Z {\xD6}\xE2";
        let (text, report) = run(&pdf(
            vec![("F1", simple_font("AkrutiKndPadmaBold"))],
            &shown("F1", short),
        ));
        assert_eq!(text, "ದಿನಾಂಕ ಪತ್ರ ದಾನ\n");
        assert!(report.rows.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn glyph_names_keep_their_codes_unless_the_list_renumbers_them() {
        let mut font = simple_font("Nudi 01 e");
        let differences = vec![Object::Integer(80), Object::Name(b"G3".to_vec())];
        font.set(
            "Encoding",
            dictionary! { "Type" => "Encoding", "Differences" => differences },
        );
        let (text, _) = run(&pdf(vec![("F1", font)], &shown("F1", KANNADA)));
        assert_eq!(text, "ಕನ್ನಡ\n");
    }

    #[test]
    fn each_layout_in_a_line_is_converted_on_its_own() {
        let mut content = b"BT /F1 12 Tf 72 700 Td (".to_vec();
        content.extend_from_slice(KANNADA);
        content.extend_from_slice(b") Tj /F2 12 Tf (d\xE2}\xE2) Tj ET");
        let (text, _) = run(&pdf(
            vec![
                ("F1", simple_font("Nudi 01 e")),
                ("F2", simple_font("AkrutiKndPadmaDemi")),
            ],
            &content,
        ));
        assert_eq!(text, "ಕನ್ನಡಕನ\n");
    }

    #[test]
    fn code_in_an_ordinary_font_is_kept() {
        let content =
            b"BT /F1 12 Tf 72 700 Td (if \\(alpha == beta && gamma != delta\\) { return true; }) Tj ET";
        let (text, report) = run(&pdf(vec![("F1", simple_font("Arial"))], content));
        assert_eq!(
            text,
            "if (alpha == beta && gamma != delta) { return true; }\n"
        );
        assert!(report.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn a_font_saved_by_q_comes_back_after_the_restore() {
        let mut content = b"BT /F2 12 Tf ET q BT /F1 12 Tf 1 0 0 1 72 650 Tm (x) Tj ET Q \
                            BT 1 0 0 1 72 600 Tm ("
            .to_vec();
        content.extend_from_slice(KANNADA);
        content.extend_from_slice(b") Tj ET");
        let fonts = vec![
            ("F1", simple_font("Arial")),
            ("F2", simple_font("BRHKannada")),
        ];
        let (text, _) = run(&pdf(fonts, &content));
        assert_eq!(text, "x\nಕನ್ನಡ\n");
    }

    #[test]
    fn digit_conventions_stay_with_their_font() {
        let content = b"BT /F1 12 Tf 72 700 Td (P\xC0 12) Tj /F2 12 Tf (P\xC0 34) Tj ET";
        let fonts = vec![
            ("F1", simple_font("Nudi 04 k")),
            ("F2", simple_font("Nudi 01 e")),
        ];
        let (text, _) = run(&pdf(fonts, content));
        assert_eq!(text, "ಕ ೧೨ಕ 34\n");
    }

    #[test]
    fn fonts_with_one_name_are_judged_apart() {
        let mut content = b"BT /F1 12 Tf 72 700 Td (".to_vec();
        content.extend_from_slice(&KANNADA.repeat(4));
        content.extend_from_slice(b") Tj /F2 12 Tf 0 -20 Td (Plain English words here) Tj ET");
        let mut other = simple_font("CIDFont+F1");
        other.set("Encoding", "MacRomanEncoding");
        let fonts = vec![("F1", simple_font("CIDFont+F1")), ("F2", other)];
        let (text, _) = run(&pdf(fonts, &content));
        assert_eq!(
            text,
            format!("{}\nPlain English words here\n", "ಕನ್ನಡ".repeat(4))
        );
    }

    #[test]
    fn huge_width_ranges_do_not_panic() {
        let (doc, font) = cid_font(Some(vec![
            Object::Integer(4_294_967_295),
            Object::Integer(4_294_967_295),
            Object::Integer(500),
            Object::Integer(4_294_967_295),
            vec![Object::Integer(250); 3].into(),
        ]));
        let bytes = pdf_with_objects(doc, vec![font], b"BT /F1 12 Tf 72 700 Td <0001> Tj ET");
        let settings = Settings::new(Direction::ToUnicode(Options::default()));
        assert!(convert_file(&bytes, &settings).is_ok());
    }

    #[test]
    fn k_fonts_give_kannada_digits() {
        let content = b"BT /F1 12 Tf 72 700 Td (P\xC0 2025) Tj ET";
        let (text, _) = run(&pdf(
            vec![("F1", simple_font("ABCDEF+Nudi 04 k,Bold"))],
            content,
        ));
        assert_eq!(text, "ಕ ೨೦೨೫\n");
    }

    #[test]
    fn only_conversion_to_unicode_is_possible() {
        let settings = Settings::new(Direction::ToNudi);
        assert!(convert_file(&pdf(vec![], b""), &settings).is_err());
        assert!(run_err(b"not a pdf"));
    }

    fn run_err(bytes: &[u8]) -> bool {
        let settings = Settings::new(Direction::ToUnicode(Options::default()));
        convert_file(bytes, &settings).is_err()
    }
}
