//! Which styles in `word/styles.xml` set a legacy font.

use std::collections::HashMap;

use anyhow::Result;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::convert::Fonts;

use super::xml::attr;

/// Whether a run's text is set in a legacy font, as far as styles say.
#[derive(Debug, Default)]
pub struct Styles {
    styles: HashMap<String, Style>,
    default_paragraph: Option<String>,
    /// From `w:docDefaults`.
    default: Option<bool>,
}

#[derive(Debug, Default)]
struct Style {
    based_on: Option<String>,
    /// Set when the style names a font for Latin text.
    legacy: Option<bool>,
}

impl Styles {
    pub fn parse(source: &str, fonts: &Fonts) -> Result<Self> {
        let mut styles = Styles::default();
        let mut reader = Reader::from_str(source);
        let mut current: Option<(String, Style)> = None;
        let mut in_defaults = false;
        loop {
            match reader.read_event()? {
                Event::Eof => break,
                Event::Start(e) | Event::Empty(e) => match e.name().as_ref() {
                    "w:docDefaults" => in_defaults = true,
                    "w:style" => {
                        let id = attr(&e, "w:styleId").unwrap_or_default();
                        let paragraph = attr(&e, "w:type").as_deref() == Some("paragraph");
                        let default = attr(&e, "w:default");
                        if paragraph && matches!(default.as_deref(), Some("1" | "true" | "on")) {
                            styles.default_paragraph = Some(id.clone());
                        }
                        current = Some((id, Style::default()));
                    }
                    "w:basedOn" => {
                        if let Some((_, style)) = current.as_mut() {
                            style.based_on = attr(&e, "w:val");
                        }
                    }
                    "w:rFonts" => {
                        let legacy = legacy_font(&e, fonts);
                        if let Some((_, style)) = current.as_mut() {
                            style.legacy = legacy.or(style.legacy);
                        } else if in_defaults {
                            styles.default = legacy.or(styles.default);
                        }
                    }
                    _ => {}
                },
                Event::End(e) => match e.name().as_ref() {
                    "w:docDefaults" => in_defaults = false,
                    "w:style" => {
                        if let Some((id, style)) = current.take() {
                            styles.styles.insert(id, style);
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        Ok(styles)
    }

    /// Whether a style, or a style it is based on, sets a legacy font.
    pub fn legacy(&self, id: &str) -> Option<bool> {
        let mut id = id;
        for _ in 0..32 {
            let style = self.styles.get(id)?;
            if style.legacy.is_some() {
                return style.legacy;
            }
            id = style.based_on.as_deref()?;
        }
        None
    }

    /// The legacy setting for a paragraph's style. A paragraph with no style, or
    /// with a style the document does not define, has the default paragraph style.
    pub fn paragraph(&self, id: Option<&str>) -> Option<bool> {
        match id {
            Some(id) if self.styles.contains_key(id) => self.legacy(id),
            _ => self.default_paragraph(),
        }
    }

    fn default_paragraph(&self) -> Option<bool> {
        self.default_paragraph
            .as_deref()
            .and_then(|id| self.legacy(id))
    }

    pub fn document_default(&self) -> Option<bool> {
        self.default
    }
}

/// Whether a `w:rFonts` element sets a legacy font for Latin text. Word draws
/// Latin-1 glyph codes with the `w:ascii` and `w:hAnsi` fonts. In each of those
/// slots a theme font, which is never legacy, wins over a named font.
pub fn legacy_font(e: &BytesStart, fonts: &Fonts) -> Option<bool> {
    let slots = [("w:ascii", "w:asciiTheme"), ("w:hAnsi", "w:hAnsiTheme")].map(|(named, theme)| {
        if attr(e, theme).is_some() {
            Some(false)
        } else {
            attr(e, named).map(|f| fonts.is_legacy(&f))
        }
    });
    if slots.contains(&Some(true)) {
        Some(true)
    } else if slots.contains(&Some(false)) {
        Some(false)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STYLES: &str = r#"<w:styles xmlns:w="w">
        <w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:asciiTheme="minorHAnsi"/></w:rPr></w:rPrDefault></w:docDefaults>
        <w:style w:type="paragraph" w:default="true" w:styleId="Normal"><w:rPr><w:rFonts w:ascii="Nudi 01 e" w:hAnsi="Nudi 01 e"/></w:rPr></w:style>
        <w:style w:type="paragraph" w:styleId="Heading1"><w:basedOn w:val="Normal"/></w:style>
        <w:style w:type="paragraph" w:styleId="English"><w:basedOn w:val="Normal"/><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/></w:rPr></w:style>
        <w:style w:type="character" w:styleId="Loop"><w:basedOn w:val="Loop"/></w:style>
    </w:styles>"#;

    #[test]
    fn theme_fonts_win_in_their_slot() {
        let fonts = Fonts::default();
        let rfonts = |attrs: &str| {
            let xml = format!("<w:rFonts {attrs}/>");
            let mut reader = Reader::from_str(&xml);
            match reader.read_event().unwrap() {
                Event::Empty(e) => legacy_font(&e.into_owned(), &fonts),
                other => panic!("{other:?}"),
            }
        };
        assert_eq!(rfonts(r#"w:ascii="Nudi 01 e""#), Some(true));
        let themed = r#"w:ascii="Nudi 01 e" w:hAnsi="Nudi 01 e" w:asciiTheme="minorHAnsi" w:hAnsiTheme="minorHAnsi""#;
        assert_eq!(rfonts(themed), Some(false));
        assert_eq!(rfonts(r#"w:cs="Tunga""#), None);
    }

    #[test]
    fn styles_inherit_fonts() {
        let styles = Styles::parse(STYLES, &Fonts::default()).unwrap();
        assert_eq!(styles.legacy("Normal"), Some(true));
        assert_eq!(styles.legacy("Heading1"), Some(true));
        assert_eq!(styles.legacy("English"), Some(false));
        assert_eq!(styles.legacy("Loop"), None);
        assert_eq!(styles.legacy("Missing"), None);
        assert_eq!(styles.paragraph(None), Some(true));
        assert_eq!(styles.paragraph(Some("Gone")), Some(true));
        assert_eq!(styles.paragraph(Some("English")), Some(false));
        assert_eq!(styles.document_default(), Some(false));
    }
}
