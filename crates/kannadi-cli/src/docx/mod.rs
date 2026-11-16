//! DOCX files. Only runs set in a legacy font are converted to Unicode, and only
//! Kannada text in other runs is converted to Nudi. A run's font comes from its own
//! `w:rFonts`, its character style, its paragraph style or the document default.
//! The body, headers, footers, footnotes, endnotes and comments are converted;
//! every other part of the package is copied unchanged.

mod part;
mod scan;
mod styles;
mod xml;

use std::io::{Cursor, Read, Write};

use anyhow::{Context, Result};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::convert::Settings;
use crate::report::Report;
use styles::Styles;

pub fn convert_file(bytes: &[u8], settings: &Settings) -> Result<(Vec<u8>, Report)> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).context("not a DOCX (zip) file")?;
    let styles = match read_part(&mut archive, "word/styles.xml")? {
        Some(source) => Styles::parse(&source, &settings.fonts).context("word/styles.xml")?,
        None => Styles::default(),
    };
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let mut report = Report::default();
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for i in 0..archive.len() {
        let name = archive.by_index(i)?.name()?.to_string();
        let converted = if has_text(&name) {
            let source = read_part(&mut archive, &name)?.unwrap_or_default();
            part::convert_part(&name, &source, &styles, settings, &mut report.rows)
                .with_context(|| name.clone())?
        } else {
            None
        };
        match converted {
            Some(xml) => {
                writer.start_file(name.as_str(), options)?;
                writer.write_all(xml.as_bytes())?;
            }
            None => writer.raw_copy_file(archive.by_index(i)?)?,
        }
    }
    Ok((writer.finish()?.into_inner(), report))
}

/// Parts that hold document text.
fn has_text(name: &str) -> bool {
    let Some(file) = name
        .strip_prefix("word/")
        .and_then(|n| n.strip_suffix(".xml"))
    else {
        return false;
    };
    let numbered = |prefix: &str| {
        file.strip_prefix(prefix)
            .is_some_and(|n| n.bytes().all(|b| b.is_ascii_digit()))
    };
    matches!(file, "document" | "footnotes" | "endnotes" | "comments")
        || numbered("header")
        || numbered("footer")
}

fn read_part(archive: &mut ZipArchive<Cursor<&[u8]>>, name: &str) -> Result<Option<String>> {
    let mut file = match archive.by_name(name) {
        Ok(file) => file,
        Err(zip::result::ZipError::FileNotFound) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let mut source = String::new();
    file.read_to_string(&mut source)
        .with_context(|| format!("{name} is not UTF-8"))?;
    Ok(Some(source))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert::{Direction, Kind};
    use kannadi::legacy::Options;

    const W: &str = "xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\"";

    fn docx(document_body: &str, styles: Option<&str>) -> Vec<u8> {
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        let options = SimpleFileOptions::default();
        writer.start_file("[Content_Types].xml", options).unwrap();
        writer.write_all(b"<Types/>").unwrap();
        writer.start_file("word/document.xml", options).unwrap();
        let document = format!("<w:document {W}><w:body>{document_body}</w:body></w:document>");
        writer.write_all(document.as_bytes()).unwrap();
        if let Some(styles) = styles {
            writer.start_file("word/styles.xml", options).unwrap();
            writer
                .write_all(format!("<w:styles {W}>{styles}</w:styles>").as_bytes())
                .unwrap();
        }
        writer.finish().unwrap().into_inner()
    }

    fn document(bytes: &[u8]) -> String {
        let mut archive = ZipArchive::new(Cursor::new(bytes)).unwrap();
        read_part(&mut archive, "word/document.xml")
            .unwrap()
            .unwrap()
    }

    fn run(bytes: &[u8], direction: Direction) -> (String, Report) {
        let (out, report) = convert_file(bytes, &Settings::new(direction)).unwrap();
        (document(&out), report)
    }

    fn to_unicode() -> Direction {
        Direction::ToUnicode(Options::default())
    }

    const NUDI: &str = "<w:rFonts w:ascii=\"Nudi 01 e\" w:hAnsi=\"Nudi 01 e\"/>";

    #[test]
    fn runs_are_read_in_their_own_layout() {
        let akruti = "<w:rFonts w:ascii=\"AkrutiKndPadma\" w:hAnsi=\"AkrutiKndPadma\"/>";
        let body = format!(
            "<w:p><w:r><w:rPr>{NUDI}</w:rPr><w:t>PÀ</w:t></w:r>\
             <w:r><w:rPr>{akruti}</w:rPr><w:t>dâ}}â</w:t></w:r></w:p>"
        );
        let (xml, report) = run(&docx(&body, None), to_unicode());
        assert!(xml.contains(">ಕ</w:t>"), "{xml}");
        assert!(xml.contains(">ಕನ</w:t>"), "{xml}");
        assert!(report.rows.is_empty(), "{:?}", report.rows);
    }

    #[test]
    fn legacy_runs_become_unicode_and_others_stay() {
        let body = format!(
            "<w:p><w:r><w:rPr>{NUDI}<w:sz w:val=\"28\"/></w:rPr><w:t>PÀ£ÀßqÀ</w:t></w:r>\
             <w:r><w:t xml:space=\"preserve\"> PÀ &amp; English</w:t></w:r></w:p>"
        );
        let (xml, report) = run(&docx(&body, None), to_unicode());
        assert!(
            xml.contains("<w:t xml:space=\"preserve\">ಕನ್ನಡ</w:t>"),
            "{xml}"
        );
        assert!(
            xml.contains("w:cs=\"Noto Sans Kannada\"/><w:sz w:val=\"28\"/><w:szCs w:val=\"28\"/>"),
            "{xml}"
        );
        assert!(
            xml.contains("<w:t xml:space=\"preserve\"> PÀ &amp; English</w:t>"),
            "{xml}"
        );
        assert!(report.is_empty());
    }

    #[test]
    fn syllables_split_across_runs_join_the_first_run() {
        let body = format!(
            "<w:p><w:r><w:rPr>{NUDI}</w:rPr><w:t>P</w:t></w:r>\
             <w:r><w:rPr>{NUDI}<w:b/></w:rPr><w:t>À£À</w:t></w:r></w:p>"
        );
        let (xml, report) = run(&docx(&body, None), to_unicode());
        assert!(report.is_empty(), "{:?}", report.rows);
        assert!(xml.contains(">ಕ</w:t>"), "{xml}");
        assert!(xml.contains(">ನ</w:t>"), "{xml}");
    }

    #[test]
    fn fonts_come_from_styles() {
        let styles = format!(
            "<w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:rPr>{NUDI}</w:rPr></w:style>\
             <w:style w:type=\"paragraph\" w:styleId=\"Body\"><w:rPr><w:rFonts w:asciiTheme=\"minorHAnsi\"/></w:rPr></w:style>\
             <w:style w:type=\"character\" w:styleId=\"En\"><w:rPr><w:rFonts w:ascii=\"Arial\"/></w:rPr></w:style>\
             <w:style w:type=\"character\" w:styleId=\"Kn\"><w:basedOn w:val=\"En\"/><w:rPr>{NUDI}</w:rPr></w:style>"
        );
        let body = "<w:p><w:r><w:t>PÀ</w:t></w:r><w:r><w:rPr><w:rStyle w:val=\"En\"/></w:rPr><w:t>PÀ</w:t></w:r></w:p>\
                    <w:p><w:pPr><w:pStyle w:val=\"Body\"/></w:pPr><w:r><w:t>PÀ</w:t></w:r>\
                    <w:r><w:rPr><w:rStyle w:val=\"Kn\"/><w:b/></w:rPr><w:t>£À</w:t></w:r></w:p>";
        let (xml, _) = run(&docx(body, Some(&styles)), to_unicode());
        assert!(
            xml.contains("<w:r><w:rPr><w:rFonts w:ascii=\"Noto Sans Kannada\""),
            "{xml}"
        );
        assert!(xml.contains(">ಕ</w:t>"), "{xml}");
        assert!(
            xml.contains("<w:rStyle w:val=\"En\"/></w:rPr><w:t>PÀ</w:t>"),
            "{xml}"
        );
        assert!(xml.contains("</w:pPr><w:r><w:t>PÀ</w:t>"), "{xml}");
        assert!(
            xml.contains("<w:rStyle w:val=\"Kn\"/><w:rFonts w:ascii=\"Noto Sans Kannada\""),
            "{xml}"
        );
        assert!(xml.contains(">ನ</w:t>"), "{xml}");
    }

    #[test]
    fn tabs_split_text_and_reports_name_the_paragraph() {
        let body = format!(
            "<w:p><w:r><w:t>x</w:t></w:r></w:p>\
             <w:p><w:r><w:rPr>{NUDI}</w:rPr><w:t>P</w:t><w:tab/><w:t>À</w:t></w:r></w:p>"
        );
        let (_, report) = run(&docx(&body, None), to_unicode());
        let kinds: Vec<Kind> = report.rows.iter().map(|r| r.kind).collect();
        assert_eq!(kinds, [Kind::Malformed, Kind::UnknownGlyph]);
        assert!(report
            .rows
            .iter()
            .all(|r| r.location == "word/document.xml, paragraph 2"));
    }

    #[test]
    fn kannada_is_split_into_legacy_runs() {
        let body = "<w:p><w:r><w:rPr><w:b/></w:rPr><w:t>Office: ಕನ್ನಡ ನಾಡು!</w:t></w:r></w:p>";
        let (xml, report) = run(&docx(body, None), Direction::ToNudi);
        assert!(report.is_empty());
        let expected = "<w:r><w:rPr><w:rFonts w:ascii=\"Nudi 01 e\" w:hAnsi=\"Nudi 01 e\" \
             w:eastAsia=\"Nudi 01 e\" w:cs=\"Nudi 01 e\"/><w:b/></w:rPr>\
             <w:t xml:space=\"preserve\">PÀ£ÀßqÀ £ÁqÀÄ</w:t></w:r>";
        assert!(xml.contains(expected), "{xml}");
        assert!(
            xml.contains("<w:t xml:space=\"preserve\">Office: </w:t>"),
            "{xml}"
        );
        assert!(
            xml.contains("<w:rPr><w:b/></w:rPr><w:t xml:space=\"preserve\">!</w:t>"),
            "{xml}"
        );
    }

    #[test]
    fn round_trip_through_nudi() {
        let body = "<w:p><w:r><w:t>ಕನ್ನಡ ನಾಡು, ಸ್ತ್ರೀ</w:t></w:r></w:p>";
        let (nudi, _) = convert_file(&docx(body, None), &Settings::new(Direction::ToNudi)).unwrap();
        let (back, report) = run(&nudi, to_unicode());
        assert!(report.is_empty(), "{:?}", report.rows);
        let text: String = back
            .split("<w:t")
            .skip(1)
            .map(|t| &t[t.find('>').unwrap() + 1..t.find("</w:t>").unwrap()])
            .collect();
        assert_eq!(text, "ಕನ್ನಡ ನಾಡು, ಸ್ತ್ರೀ");
    }

    #[test]
    fn odd_but_valid_xml_is_handled() {
        let bom = format!("\u{FEFF}<w:document {W}><w:body><w:p><w:r><w:rPr>{NUDI}</w:rPr><w:t>PÀ</w:t></w:r></w:p></w:body></w:document>");
        let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file("word/document.xml", SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bom.as_bytes()).unwrap();
        let bytes = writer.finish().unwrap().into_inner();
        let (xml, _) = run(&bytes, to_unicode());
        assert!(
            xml.starts_with('\u{FEFF}') && xml.contains(">ಕ</w:t>"),
            "{xml}"
        );

        let open_close =
            "<w:p><w:r><w:rPr><w:rFonts w:ascii=\"Nudi 01 e\" w:hAnsi=\"Nudi 01 e\"></w:rFonts>\
                          <w:sz w:val=\"28\"></w:sz></w:rPr><w:t>PÀ</w:t></w:r></w:p>";
        let (xml, _) = run(&docx(open_close, None), to_unicode());
        assert!(!xml.contains("</w:rFonts>"), "{xml}");
        assert!(
            xml.contains("<w:sz w:val=\"28\"></w:sz><w:szCs w:val=\"28\"/>"),
            "{xml}"
        );

        let nudi_normal = format!(
            "<w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"><w:rPr>{NUDI}</w:rPr></w:style>"
        );
        let nested = "<w:p><w:r><w:rPr><w:r><w:t>PÀ</w:t></w:r></w:rPr><w:t>PÀ</w:t></w:r></w:p>";
        let (xml, _) = run(&docx(nested, Some(&nudi_normal)), to_unicode());
        assert!(xml.contains("<w:r><w:t>PÀ</w:t></w:r>"), "{xml}");
        assert!(
            xml.contains("<w:t xml:space=\"preserve\">ಕ</w:t></w:r></w:p>"),
            "{xml}"
        );
    }

    #[test]
    fn tracked_changes_are_read_like_word_does() {
        let styles = format!(
            "<w:style w:type=\"paragraph\" w:styleId=\"Kn\"><w:rPr>{NUDI}</w:rPr></w:style>\
             <w:style w:type=\"paragraph\" w:default=\"1\" w:styleId=\"Normal\"/>"
        );
        let body = "<w:p><w:pPr><w:pStyle w:val=\"Kn\"/><w:pPrChange w:id=\"1\"><w:pPr><w:pStyle w:val=\"Normal\"/></w:pPr></w:pPrChange></w:pPr>\
                    <w:r><w:t>PÀ</w:t></w:r><w:del w:id=\"2\"><w:r><w:delText>£À</w:delText></w:r></w:del></w:p>";
        let (xml, _) = run(&docx(body, Some(&styles)), to_unicode());
        assert!(xml.contains(">ಕ</w:t>"), "{xml}");
        assert!(
            xml.contains("<w:delText xml:space=\"preserve\">ನ</w:delText>"),
            "{xml}"
        );
    }

    #[test]
    fn a_zero_at_a_run_boundary_reads_as_in_one_run() {
        let body = format!(
            "<w:p><w:r><w:rPr>{NUDI}</w:rPr><w:t>¥ÀæÁgÀ</w:t></w:r><w:r><w:rPr>{NUDI}</w:rPr><w:t>0¨sÀ</w:t></w:r></w:p>"
        );
        let (xml, _) = run(&docx(&body, None), to_unicode());
        let text: String = xml
            .split("<w:t")
            .skip(1)
            .map(|t| &t[t.find('>').unwrap() + 1..t.find("</w:t>").unwrap()])
            .collect();
        assert_eq!(text, "ಪ್ರಾರಂಭ");
    }

    #[test]
    fn bold_italic_and_size_follow_the_script() {
        let body = format!("<w:p><w:r><w:rPr>{NUDI}<w:b/><w:i w:val=\"1\"/><w:sz w:val=\"28\"/></w:rPr><w:t>PÀ</w:t></w:r></w:p>");
        let (xml, _) = run(&docx(&body, None), to_unicode());
        assert!(xml.contains("<w:b/><w:bCs/><w:i w:val=\"1\"/><w:iCs w:val=\"1\"/><w:sz w:val=\"28\"/><w:szCs w:val=\"28\"/>"), "{xml}");
        let body = "<w:p><w:r><w:rPr><w:bCs/><w:sz w:val=\"22\"/><w:szCs w:val=\"32\"/></w:rPr><w:t>ಕ</w:t></w:r></w:p>";
        let (xml, _) = run(&docx(body, None), Direction::ToNudi);
        assert!(xml.contains("<w:b/><w:bCs/><w:sz w:val=\"32\"/><w:szCs w:val=\"32\"/></w:rPr><w:t xml:space=\"preserve\">PÀ"), "{xml}");
    }

    #[test]
    fn text_box_fallback_copies_are_converted_but_not_reported_twice() {
        let box_text = |text: &str| {
            format!("<w:txbxContent><w:p><w:r><w:rPr>{NUDI}</w:rPr><w:t>{text}</w:t></w:r></w:p></w:txbxContent>")
        };
        let text_box = |choice: &str, fallback: &str| {
            format!(
                "<w:p><w:r><mc:AlternateContent><mc:Choice>{}</mc:Choice>\
                 <mc:Fallback>{}</mc:Fallback></mc:AlternateContent></w:r></w:p>",
                box_text(choice),
                box_text(fallback)
            )
        };
        let (_, report) = run(&docx(&text_box("P", "P"), None), to_unicode());
        assert_eq!(report.rows.len(), 1, "{:?}", report.rows);
        assert_eq!(report.rows[0].location, "word/document.xml, paragraph 2");
        let (_, report) = run(&docx(&text_box("PÀ", "PÀ\u{F1}"), None), to_unicode());
        assert_eq!(report.rows.len(), 1, "{:?}", report.rows);
        assert_eq!(report.rows[0].kind, Kind::UnknownGlyph);
    }

    #[test]
    fn text_never_moves_across_revisions_or_branches() {
        let body = format!(
            "<w:p><w:r><w:rPr>{NUDI}</w:rPr><w:t>PÀ</w:t></w:r>\
             <w:del w:id=\"1\"><w:r><w:rPr>{NUDI}</w:rPr><w:delText>A</w:delText></w:r></w:del></w:p>"
        );
        let (xml, _) = run(&docx(&body, None), to_unicode());
        assert!(xml.contains(">ಕ</w:t>"), "{xml}");
        assert!(xml.contains(">ಂ</w:delText>"), "{xml}");

        let branch = |text: &str| format!("<w:r><w:rPr>{NUDI}</w:rPr><w:t>{text}</w:t></w:r>");
        let body = format!(
            "<w:p><mc:AlternateContent><mc:Choice>{}</mc:Choice><mc:Fallback>{}</mc:Fallback></mc:AlternateContent></w:p>",
            branch("PÀ"),
            branch("PÀ\u{F1}")
        );
        let (xml, report) = run(&docx(&body, None), to_unicode());
        assert_eq!(xml.matches(">ಕ").count(), 2, "{xml}");
        assert_eq!(report.rows.len(), 1, "{:?}", report.rows);
        assert_eq!(report.rows[0].kind, Kind::UnknownGlyph);
        let same = format!(
            "<w:p><mc:AlternateContent><mc:Choice>{}</mc:Choice><mc:Fallback>{}</mc:Fallback></mc:AlternateContent></w:p>",
            branch("PÀ\u{F1}"),
            branch("PÀ\u{F1}")
        );
        let (_, report) = run(&docx(&same, None), to_unicode());
        assert_eq!(report.rows.len(), 1, "{:?}", report.rows);
    }

    #[test]
    fn other_parts_are_copied_and_headers_converted() {
        assert!(has_text("word/header2.xml"));
        assert!(has_text("word/footnotes.xml"));
        assert!(!has_text("word/styles.xml"));
        assert!(!has_text("word/headerx.xml"));
        let out = convert_file(&docx("<w:p/>", None), &Settings::new(to_unicode())).unwrap();
        let mut archive = ZipArchive::new(Cursor::new(out.0.as_slice())).unwrap();
        assert_eq!(
            read_part(&mut archive, "[Content_Types].xml")
                .unwrap()
                .unwrap(),
            "<Types/>"
        );
    }

    #[test]
    fn non_zip_input_is_an_error() {
        assert!(convert_file(b"not a zip", &Settings::new(to_unicode())).is_err());
    }
}
