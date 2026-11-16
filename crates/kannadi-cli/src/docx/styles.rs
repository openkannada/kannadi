//! Which styles in `word/styles.xml` set a legacy font.

use std::collections::HashMap;

use anyhow::Result;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::convert::Fonts;

use super::xml;

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
                        if paragraph && attr(&e, "w:default").as_deref() == Some("1") {
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

    /// The legacy setting for a paragraph with no style of its own.
    pub fn default_paragraph(&self) -> Option<bool> {
        self.default_paragraph
            .as_deref()
            .and_then(|id| self.legacy(id))
    }

    pub fn document_default(&self) -> Option<bool> {
        self.default
    }
}

/// Whether a `w:rFonts` element sets a legacy font for Latin text. Word draws
/// Latin-1 glyph codes with the `w:ascii` and `w:hAnsi` fonts; a theme font in
/// their place is never legacy.
pub fn legacy_font(e: &BytesStart, fonts: &Fonts) -> Option<bool> {
    let named = ["w:ascii", "w:hAnsi"].map(|a| attr(e, a));
    let themed = ["w:asciiTheme", "w:hAnsiTheme"].map(|a| attr(e, a).is_some());
    if named
        .iter()
        .any(|n| n.as_deref().is_some_and(|f| fonts.is_legacy(f)))
    {
        Some(true)
    } else if named.iter().any(Option::is_some) || themed.iter().any(|&t| t) {
        Some(false)
    } else {
        None
    }
}

/// An attribute value with references decoded.
pub fn attr(e: &BytesStart, name: &str) -> Option<String> {
    let a = e.try_get_attribute(name).ok()??;
    Some(xml::decode(&a.value))
}

#[cfg(test)]
mod tests {
    use super::*;

    const STYLES: &str = r#"<w:styles xmlns:w="w">
        <w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:asciiTheme="minorHAnsi"/></w:rPr></w:rPrDefault></w:docDefaults>
        <w:style w:type="paragraph" w:default="1" w:styleId="Normal"><w:rPr><w:rFonts w:ascii="Nudi 01 e" w:hAnsi="Nudi 01 e"/></w:rPr></w:style>
        <w:style w:type="paragraph" w:styleId="Heading1"><w:basedOn w:val="Normal"/></w:style>
        <w:style w:type="paragraph" w:styleId="English"><w:basedOn w:val="Normal"/><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/></w:rPr></w:style>
        <w:style w:type="character" w:styleId="Loop"><w:basedOn w:val="Loop"/></w:style>
    </w:styles>"#;

    #[test]
    fn styles_inherit_fonts() {
        let styles = Styles::parse(STYLES, &Fonts::default()).unwrap();
        assert_eq!(styles.legacy("Normal"), Some(true));
        assert_eq!(styles.legacy("Heading1"), Some(true));
        assert_eq!(styles.legacy("English"), Some(false));
        assert_eq!(styles.legacy("Loop"), None);
        assert_eq!(styles.legacy("Missing"), None);
        assert_eq!(styles.default_paragraph(), Some(true));
        assert_eq!(styles.document_default(), Some(false));
    }
}
