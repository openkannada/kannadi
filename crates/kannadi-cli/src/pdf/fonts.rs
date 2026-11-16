//! Which fonts in a PDF hold legacy text, judged once all their text is known.

use std::collections::HashMap;
use std::rc::Rc;

use kannadi::legacy::{best_layout, to_unicode, windows_1252_byte, Layout, Options};
use lopdf::{Dictionary, Document, Object};

use crate::convert::Settings;
use crate::fonts::has_kannada_digits;

use super::cmap::CMap;
use super::widths::Widths;
use super::{resolve, LIMIT};

pub const RENUMBERED: &str = "named as a legacy font, but its glyph codes do not follow the \
     layout and its character map does not help; the PDF maker may have renumbered them";
pub const OTHER_LAYOUT: &str = "looks like legacy Kannada in a layout this tool does not know";
pub const RENUMBERED_SUBSET: &str = "its glyph codes were renumbered and the PDF has no usable \
     character map, so its text cannot be read";

/// Which reading of a font's strings holds its legacy codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The string bytes themselves, or the encoding's reading of them.
    Raw,
    /// The font's ToUnicode map, for subsets whose codes were renumbered.
    Mapped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Legacy(Source),
    Other,
    /// Legacy text that does not convert, for the reason given.
    Unreadable(&'static str),
}

#[derive(Debug)]
pub struct Font {
    pub name: String,
    /// The layout the font's name gives, if it is a legacy font.
    named: Option<Layout>,
    /// The layout its text is read in: the named one, or the one its text fits.
    read_as: Layout,
    /// The font maps codes with its own glyph list and no ToUnicode map.
    custom_codes: bool,
    pub kannada_digits: bool,
    /// The raw reading of all text shown in the font.
    text: String,
    /// The ToUnicode reading, for a 1-byte font named as legacy that has a map.
    mapped: String,
    pub verdict: Verdict,
}

impl Font {
    pub fn layout(&self) -> Layout {
        self.read_as
    }
}

#[derive(Debug, Default)]
pub struct Fonts {
    pub fonts: Vec<Font>,
    /// By [`font_key`]: copies of 1 font on many pages pool their text, and
    /// different fonts that share a name stay apart.
    by_key: HashMap<(String, u64), usize>,
    /// By the address of the font's dictionary, which lives as long as the PDF.
    widths: HashMap<usize, Rc<Widths>>,
    /// By the address of the font's dictionary.
    cmaps: HashMap<usize, Option<Rc<CMap>>>,
}

impl Fonts {
    pub fn index(&mut self, doc: &Document, dict: &Dictionary, settings: &Settings) -> usize {
        let key = font_key(doc, dict);
        if let Some(&i) = self.by_key.get(&key) {
            return i;
        }
        let name = key.0.clone();
        let plain = plain_name(&name);
        let named = settings.fonts.layout(plain);
        let custom_codes = dict.get(b"ToUnicode").is_err() && has_differences(doc, dict);
        self.fonts.push(Font {
            named,
            read_as: named.unwrap_or(Layout::Nudi),
            custom_codes,
            kannada_digits: has_kannada_digits(plain),
            name,
            text: String::new(),
            mapped: String::new(),
            verdict: Verdict::Other,
        });
        self.by_key.insert(key, self.fonts.len() - 1);
        self.fonts.len() - 1
    }

    pub fn is_named_legacy(&self, index: usize) -> bool {
        self.fonts[index].named.is_some()
    }

    pub fn widths(&mut self, doc: &Document, dict: &Dictionary) -> Rc<Widths> {
        let key = std::ptr::from_ref(dict) as usize;
        Rc::clone(
            self.widths
                .entry(key)
                .or_insert_with(|| Rc::new(Widths::of(doc, dict))),
        )
    }

    /// The font's ToUnicode map, read once per document.
    pub fn cmap(&mut self, doc: &Document, dict: &Dictionary) -> Option<Rc<CMap>> {
        let key = std::ptr::from_ref(dict) as usize;
        self.cmaps
            .entry(key)
            .or_insert_with(|| {
                let stream = resolve(doc, dict.get(b"ToUnicode").ok()?)?
                    .as_stream()
                    .ok()?;
                let bytes = stream.decompressed_content_with_limit(LIMIT).ok()?;
                Some(Rc::new(CMap::parse(&bytes))).filter(|m| !m.is_empty())
            })
            .clone()
    }

    pub fn add_text(&mut self, index: usize, raw: &str, mapped: Option<&str>) {
        let font = &mut self.fonts[index];
        font.text.push_str(raw);
        if let Some(text) = mapped {
            font.mapped.push_str(text);
        }
    }

    /// Decides which fonts hold legacy text, once all their text is known.
    pub fn decide(&mut self) {
        for font in &mut self.fonts {
            let layout = font.layout();
            let raw = Judgment::of(&font.text, layout);
            let renumbered = looks_renumbered(&font.text);
            font.verdict = if font.named.is_some() {
                let mapped = Judgment::of(&font.mapped, layout);
                // Short text: the map wins when only its reading looks like legacy codes.
                let mapped_usable = mapped.clean()
                    || (mapped.short()
                        && mapped.flawless()
                        && looks_legacy(&font.mapped)
                        && !looks_legacy(&font.text));
                // Akruti draws letters at codes renumbered text uses for symbols, so
                // short text that reads with no problems is taken as it is.
                let raw_usable = raw.short() && (raw.flawless() || !renumbered);
                if raw.clean() || (raw_usable && !mapped_usable) {
                    Verdict::Legacy(Source::Raw)
                } else if mapped_usable {
                    Verdict::Legacy(Source::Mapped)
                } else {
                    Verdict::Unreadable(RENUMBERED)
                }
            } else if looks_legacy(&font.text) {
                let best = best_layout(&font.text);
                let judged = Judgment::of(&font.text, best);
                if judged.clean() || (judged.short() && judged.flawless()) {
                    font.read_as = best;
                    Verdict::Legacy(Source::Raw)
                } else if raw.short() {
                    Verdict::Other
                } else {
                    Verdict::Unreadable(OTHER_LAYOUT)
                }
            } else if font.custom_codes && renumbered {
                Verdict::Unreadable(RENUMBERED_SUBSET)
            } else {
                Verdict::Other
            };
        }
    }

    pub fn skipped(&self) -> impl Iterator<Item = (&str, &str)> {
        self.fonts.iter().filter_map(|f| match f.verdict {
            Verdict::Unreadable(reason) => Some((f.name.as_str(), reason)),
            _ => None,
        })
    }
}

/// Whether a simple font's encoding has a `Differences` list.
fn has_differences(doc: &Document, dict: &Dictionary) -> bool {
    dict.get(b"Encoding")
        .ok()
        .and_then(|e| resolve(doc, e))
        .and_then(|e| e.as_dict().ok())
        .is_some_and(|d| d.get(b"Differences").is_ok())
}

/// A font's name and a fingerprint of how it maps codes: its type, its encoding
/// and its ToUnicode map.
fn font_key(doc: &Document, dict: &Dictionary) -> (String, u64) {
    use std::hash::{Hash, Hasher};
    let name = dict
        .get(b"BaseFont")
        .and_then(Object::as_name)
        .map(|n| String::from_utf8_lossy(n).to_string())
        .unwrap_or_default();
    let entry = |key: &[u8]| dict.get(key).ok().and_then(|o| resolve(doc, o));
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for key in [&b"Subtype"[..], b"Encoding", b"FontDescriptor"] {
        format!("{:?}", entry(key)).hash(&mut hasher);
    }
    if let Some(Object::Stream(s)) = entry(b"ToUnicode") {
        s.content.hash(&mut hasher);
    }
    (name, hasher.finish())
}

/// The font name without a subset tag (`ABCDEF+`) or a style suffix (`,Bold`).
pub fn plain_name(name: &str) -> &str {
    let name = match name.split_once('+') {
        Some((tag, rest)) if tag.len() == 6 && tag.bytes().all(|b| b.is_ascii_uppercase()) => rest,
        _ => name,
    };
    name.split(',').next().unwrap_or(name)
}

/// How a font's text converts in a layout.
struct Judgment {
    syllables: usize,
    problems: usize,
}

impl Judgment {
    fn of(text: &str, layout: Layout) -> Self {
        let converted = to_unicode(text, layout, &Options::default());
        Judgment {
            syllables: kannadi::count_aksharas(&converted.text),
            problems: converted.unknown.len() + converted.malformed.len(),
        }
    }

    /// Fewer than 10 syllables: too little text to judge by its problem rate.
    fn short(&self) -> bool {
        self.syllables < 10
    }

    /// At least 10 syllables, with problems in at most 1 in 10.
    fn clean(&self) -> bool {
        !self.short() && self.problems * 10 <= self.syllables
    }

    /// Some syllables and no problems.
    fn flawless(&self) -> bool {
        self.syllables > 0 && self.problems == 0
    }
}

/// Whether text reads as symbols, as when a font's codes were renumbered from `!`:
/// at least 1 in 7 of its letters and symbols are symbols that prose, numbers,
/// web addresses and form blanks rarely use. A run of 1 symbol counts once.
pub fn looks_renumbered(text: &str) -> bool {
    let letters = text.chars().filter(|c| c.is_alphabetic()).count();
    let mut symbols = 0;
    let mut last = None;
    for c in text.chars() {
        if "!\"#$%&+<=>?[\\]^`{|}~".contains(c) && last != Some(c) {
            symbols += 1;
        }
        last = Some(c);
    }
    letters >= 10 && symbols * 7 >= letters + symbols
}

/// Whether at least 3 in 10 visible characters are Windows-1252 characters above
/// ASCII, as in legacy Kannada and rarely in other text.
pub fn looks_legacy(text: &str) -> bool {
    let visible: Vec<char> = text.chars().filter(|c| !c.is_whitespace()).collect();
    let high = visible
        .iter()
        .filter(|&&c| !c.is_ascii() && windows_1252_byte(c).is_some())
        .count();
    visible.len() >= 4 && high * 10 >= visible.len() * 3
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renumbered_text_is_told_from_forms_addresses_and_code() {
        assert!(looks_renumbered(
            "!\"! #$$ %&%&%& )*+,-*!*.*! /-01&-01& KLMC'KLMC' WXYZQUQV[R\\Q]"
        ));
        for text in [
            "www.advgen.kar.nic.in 560 001. 16000-400-17200 85/2015-16, 09.07.2015",
            "Sultan & Another Vs. U.P. Public Service Commission (C) No. 2/2024",
            "Name: ______________ Address: ____ Date: ____",
            "Contact info@kar.nic.in for forms #12 and the annexure",
        ] {
            assert!(!looks_renumbered(text), "{text}");
        }
    }

    #[test]
    fn names_lose_their_subset_tag_and_style() {
        assert_eq!(plain_name("ABCDEF+Nudi 04 k,Bold"), "Nudi 04 k");
        assert_eq!(plain_name("CIDFont+F1"), "CIDFont+F1");
    }

    #[test]
    fn short_legacy_text_counts() {
        assert!(looks_legacy("¨sÁUÀ"));
        assert!(!looks_legacy("José Müller"));
    }
}
