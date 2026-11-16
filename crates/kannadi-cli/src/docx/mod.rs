//! DOCX files. Only runs set in a legacy font are converted to Unicode, and only
//! Kannada text in other runs is converted to Nudi. A run's font comes from its own
//! `w:rFonts`, its character style, its paragraph style or the document default.
//! The body, headers, footers, footnotes, endnotes and comments are converted;
//! every other part of the package is copied unchanged.

mod part;
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
