//! Which styles in `word/styles.xml` set a legacy font.

use std::collections::HashMap;

use anyhow::Result;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use kannadi::legacy::Layout;

use crate::convert::Fonts;

use super::xml::attr;

/// What a font setting says about the Latin text of a run.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FontSetting {
    /// It names no Latin font, so the text inherits the setting.
    #[default]
    Inherit,
    /// A font that is not legacy.
    Other,
    /// A legacy font in this layout.
    Legacy(Layout),
}

impl FontSetting {
    /// This setting, or the one `next` gives when this one inherits.
    pub fn or_else(self, next: impl FnOnce() -> FontSetting) -> FontSetting {
        match self {
            FontSetting::Inherit => next(),
            set => set,
        }
    }

    pub fn layout(self) -> Option<Layout> {
        match self {
            FontSetting::Legacy(layout) => Some(layout),
            _ => None,
        }
    }
}

/// Whether a run's text is set in a legacy font, as far as styles say.
#[derive(Debug, Default)]
pub struct Styles {
    styles: HashMap<String, Style>,
    default_paragraph: Option<String>,
    /// From `w:docDefaults`.
    default: FontSetting,
}

#[derive(Debug, Default)]
struct Style {
    based_on: Option<String>,
    /// Set when the style names a font for Latin text.
    font: FontSetting,
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
                Event::Empty(e) if e.name().as_ref() == "w:style" => {
                    let id = attr(&e, "w:styleId").unwrap_or_default();
                    let paragraph = attr(&e, "w:type").as_deref() == Some("paragraph");
                    let default = attr(&e, "w:default");
                    if paragraph && matches!(default.as_deref(), Some("1" | "true" | "on")) {
                        styles.default_paragraph = Some(id.clone());
                    }
                    styles.styles.insert(id, Style::default());
                }
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
                        let font = font_setting(&e, fonts);
                        if let Some((_, style)) = current.as_mut() {
                            style.font = font.or_else(|| style.font);
                        } else if in_defaults {
                            styles.default = font.or_else(|| styles.default);
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

    /// The font setting of a style, or of the style it is based on.
    pub fn font(&self, id: &str) -> FontSetting {
        let mut id = id;
        for _ in 0..32 {
            let Some(style) = self.styles.get(id) else {
                break;
            };
            if style.font != FontSetting::Inherit {
                return style.font;
            }
            let Some(next) = style.based_on.as_deref() else {
                break;
            };
            id = next;
        }
        FontSetting::Inherit
    }

    /// The font setting of a paragraph's style. A paragraph with no style, or
    /// with a style the document does not define, has the default paragraph style.
    pub fn paragraph(&self, id: Option<&str>) -> FontSetting {
        match id {
            Some(id) if self.styles.contains_key(id) => self.font(id),
            _ => self.default_paragraph(),
        }
    }

    fn default_paragraph(&self) -> FontSetting {
        self.default_paragraph
            .as_deref()
            .map_or(FontSetting::Inherit, |id| self.font(id))
    }

    pub fn document_default(&self) -> FontSetting {
        self.default
    }
}

/// Whether a `w:rFonts` element sets a legacy font for Latin text, and in which
/// layout. Word draws Latin-1 glyph codes with the `w:ascii` and `w:hAnsi` fonts.
/// In each of those slots a theme font, which is never legacy, wins over a named
/// font. A legacy font in either slot wins.
pub fn font_setting(e: &BytesStart, fonts: &Fonts) -> FontSetting {
    let slots = [("w:ascii", "w:asciiTheme"), ("w:hAnsi", "w:hAnsiTheme")].map(|(named, theme)| {
        if attr(e, theme).is_some() {
            FontSetting::Other
        } else {
            attr(e, named).map_or(FontSetting::Inherit, |f| {
                fonts
                    .layout(&f)
                    .map_or(FontSetting::Other, FontSetting::Legacy)
            })
        }
    });
    let legacy = slots.iter().find(|s| matches!(s, FontSetting::Legacy(_)));
    let set = slots.iter().find(|&&s| s != FontSetting::Inherit);
    legacy.or(set).copied().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::FontSetting::{Inherit, Legacy, Other};
    use super::*;

    const STYLES: &str = r#"<w:styles xmlns:w="w">
        <w:docDefaults><w:rPrDefault><w:rPr><w:rFonts w:asciiTheme="minorHAnsi"/></w:rPr></w:rPrDefault></w:docDefaults>
        <w:style w:type="paragraph" w:default="true" w:styleId="Normal"><w:rPr><w:rFonts w:ascii="Nudi 01 e" w:hAnsi="Nudi 01 e"/></w:rPr></w:style>
        <w:style w:type="paragraph" w:styleId="Heading1"><w:basedOn w:val="Normal"/></w:style>
        <w:style w:type="paragraph" w:styleId="English"><w:basedOn w:val="Normal"/><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial"/></w:rPr></w:style>
        <w:style w:type="character" w:styleId="Loop"><w:basedOn w:val="Loop"/></w:style>
        <w:style w:type="paragraph" w:styleId="Plain"/>
    </w:styles>"#;

    #[test]
    fn theme_fonts_win_in_their_slot() {
        let fonts = Fonts::default();
        let rfonts = |attrs: &str| {
            let xml = format!("<w:rFonts {attrs}/>");
            let mut reader = Reader::from_str(&xml);
            match reader.read_event().unwrap() {
                Event::Empty(e) => font_setting(&e.into_owned(), &fonts),
                other => panic!("{other:?}"),
            }
        };
        assert_eq!(rfonts(r#"w:ascii="Nudi 01 e""#), Legacy(Layout::Nudi));
        assert_eq!(
            rfonts(r#"w:ascii="Arial" w:hAnsi="AkrutiKndPadmaDemi""#),
            Legacy(Layout::Akruti)
        );
        let themed = r#"w:ascii="Nudi 01 e" w:hAnsi="Nudi 01 e" w:asciiTheme="minorHAnsi" w:hAnsiTheme="minorHAnsi""#;
        assert_eq!(rfonts(themed), Other);
        assert_eq!(rfonts(r#"w:cs="Tunga""#), Inherit);
    }

    #[test]
    fn styles_inherit_fonts() {
        let styles = Styles::parse(STYLES, &Fonts::default()).unwrap();
        assert_eq!(styles.font("Normal"), Legacy(Layout::Nudi));
        assert_eq!(styles.font("Heading1"), Legacy(Layout::Nudi));
        assert_eq!(styles.font("English"), Other);
        assert_eq!(styles.font("Loop"), Inherit);
        assert_eq!(styles.font("Missing"), Inherit);
        assert_eq!(styles.paragraph(None), Legacy(Layout::Nudi));
        assert_eq!(styles.paragraph(Some("Gone")), Legacy(Layout::Nudi));
        assert_eq!(styles.paragraph(Some("English")), Other);
        assert_eq!(styles.paragraph(Some("Plain")), Inherit);
        assert_eq!(styles.document_default(), Other);
    }
}
