//! Which fonts in a PDF hold legacy text, judged once all their text is known.

use std::collections::HashMap;
use std::rc::Rc;

use kannadi::legacy::{nudi_to_unicode, windows_1252_byte, Options};
use lopdf::{Dictionary, Document, Object};

use crate::convert::Settings;
use crate::fonts::has_kannada_digits;

use super::widths::Widths;

pub const RENUMBERED: &str = "named as a Nudi or Baraha font, but its glyph codes do not follow \
     the layout and its character map does not help; the PDF maker may have renumbered them";
pub const OTHER_LAYOUT: &str = "looks like legacy Kannada in a layout other than Nudi and Baraha";
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
    named_legacy: bool,
    /// The font maps codes with its own glyph list and no ToUnicode map.
    custom_codes: bool,
    pub kannada_digits: bool,
    /// The raw reading of all text shown in the font.
    text: String,
    /// The ToUnicode reading, for a 1-byte font named as legacy that has a map.
    mapped: Option<String>,
    pub verdict: Verdict,
}

#[derive(Debug, Default)]
pub struct Fonts {
    pub fonts: Vec<Font>,
    /// By [`font_key`]: copies of 1 font on many pages pool their text, and
    /// different fonts that share a name stay apart.
    by_key: HashMap<(String, u64), usize>,
    /// By the address of the font's dictionary, which lives as long as the PDF.
    widths: HashMap<usize, Rc<Widths>>,
}

impl Fonts {
    pub fn index(
        &mut self,
        doc: &Document,
        dict: &Dictionary,
        settings: &Settings,
        mapped: bool,
    ) -> usize {
        let key = font_key(doc, dict);
        if let Some(&i) = self.by_key.get(&key) {
            return i;
        }
        let name = key.0.clone();
        let plain = plain_name(&name);
        let named_legacy = settings.fonts.is_legacy(plain);
        let custom_codes = dict.get(b"ToUnicode").is_err() && has_differences(doc, dict);
        self.fonts.push(Font {
            named_legacy,
            custom_codes,
            kannada_digits: has_kannada_digits(plain),
            name,
            text: String::new(),
            mapped: mapped.then(String::new),
            verdict: Verdict::Other,
        });
        self.by_key.insert(key, self.fonts.len() - 1);
        self.fonts.len() - 1
    }

    pub fn is_named_legacy(&self, index: usize) -> bool {
        self.fonts[index].named_legacy
    }

    pub fn widths(&mut self, doc: &Document, dict: &Dictionary) -> Rc<Widths> {
        let key = std::ptr::from_ref(dict) as usize;
        Rc::clone(
            self.widths
                .entry(key)
                .or_insert_with(|| Rc::new(Widths::of(doc, dict))),
        )
    }

    pub fn add_text(&mut self, index: usize, raw: &str, mapped: Option<&str>) {
        let font = &mut self.fonts[index];
        font.text.push_str(raw);
        if let (Some(all), Some(text)) = (font.mapped.as_mut(), mapped) {
            all.push_str(text);
        }
    }

    /// Decides which fonts hold legacy text, once all their text is known.
    pub fn decide(&mut self) {
        for font in &mut self.fonts {
            let raw = judge(&font.text);
            let renumbered = looks_renumbered(&font.text);
            let mapped_clean = font
                .mapped
                .as_deref()
                .is_some_and(|m| judge(m) == Judged::Clean);
            font.verdict = if font.named_legacy {
                let raw_usable = raw == Judged::Clean
                    || (raw == Judged::TooShort && !renumbered && !mapped_clean);
                if raw_usable {
                    Verdict::Legacy(Source::Raw)
                } else if mapped_clean {
                    Verdict::Legacy(Source::Mapped)
                } else {
                    Verdict::Unreadable(RENUMBERED)
                }
            } else if looks_legacy(&font.text) {
                match raw {
                    Judged::Unclean => Verdict::Unreadable(OTHER_LAYOUT),
                    _ => Verdict::Legacy(Source::Raw),
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
    let encoding = match dict.get(b"Encoding") {
        Ok(Object::Reference(id)) => doc.get_object(*id).ok(),
        Ok(other) => Some(other),
        Err(_) => None,
    };
    encoding
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
    let resolve = |o: &Object| match o {
        Object::Reference(id) => doc.get_object(*id).ok().cloned(),
        other => Some(other.clone()),
    };
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for entry in [&b"Subtype"[..], b"Encoding", b"FontDescriptor"] {
        format!("{:?}", dict.get(entry).ok().and_then(resolve)).hash(&mut hasher);
    }
    if let Some(Object::Stream(s)) = dict.get(b"ToUnicode").ok().and_then(resolve) {
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

#[derive(Debug, PartialEq, Eq)]
enum Judged {
    /// Problems in at most 1 syllable in 10.
    Clean,
    Unclean,
    /// Fewer than 10 syllables.
    TooShort,
}

fn judge(text: &str) -> Judged {
    let converted = nudi_to_unicode(text, &Options::default());
    let syllables = kannadi::count_aksharas(&converted.text);
    let problems = converted.unknown.len() + converted.malformed.len();
    match syllables {
        0..=9 => Judged::TooShort,
        _ if problems * 10 <= syllables => Judged::Clean,
        _ => Judged::Unclean,
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
