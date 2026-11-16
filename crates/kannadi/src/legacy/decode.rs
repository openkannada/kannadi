use std::ops::Range;
use std::sync::OnceLock;

use unicode_normalization::UnicodeNormalization;

use crate::chars::{class, Class, ANUSVARA, VIRAMA};
use crate::lookup::Lookup;
use crate::numerals::to_kannada_digit;

use super::table::{rows, Entry};
use super::{aspirated, is_passthrough, Layout};

/// Settings for [`to_unicode`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    /// Write digits as Kannada digits. Use this for text set in the `k` variants of
    /// the Nudi fonts, which draw Kannada digits at the ASCII digit positions.
    pub kannada_digits: bool,
    /// Turn a zero typed between a Kannada letter or vowel sign and the next letter
    /// into an anusvara (ಂ), a common typing habit in legacy documents: ಪ್ರಾರ0ಭ
    /// becomes ಪ್ರಾರಂಭ.
    pub zero_as_anusvara: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            kannada_digits: false,
            zero_as_anusvara: true,
        }
    }
}

/// A glyph the converter could not place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unknown {
    /// Byte offset of the glyph in the input.
    pub offset: usize,
    /// The glyph, as it appears in the input.
    pub glyph: char,
}

/// The result of a conversion.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conversion {
    /// The Unicode text. Unknown glyphs are kept as they were.
    pub text: String,
    /// Glyphs the converter could not place: glyphs with no known meaning, and
    /// known glyphs in a position they cannot take, such as a second arkavattu.
    pub unknown: Vec<Unknown>,
    /// Byte ranges in `text` that are not valid syllables: a mark with no letter,
    /// 2 vowel signs on 1 consonant, a sign on a vowel letter, or a consonant shape
    /// typed without its sign. A clean conversion has none.
    pub malformed: Vec<Range<usize>>,
}

impl Conversion {
    /// True if every glyph was converted and every syllable is well formed.
    pub fn is_clean(&self) -> bool {
        self.unknown.is_empty() && self.malformed.is_empty()
    }
}

/// Converts text typed in a legacy font with glyph layout `layout` to Unicode
/// Kannada.
///
/// The input is the text as stored, read as Windows-1252: Word documents and web
/// pages give it that way. For raw bytes, use [`decode_windows_1252`](super::decode_windows_1252) first.
/// Only pass text that was set in a legacy font; English text in the same document
/// would be converted too.
///
/// ```
/// use kannadi::legacy::{to_unicode, Layout, Options};
///
/// let converted = to_unicode("PÀ£ÀßqÀ", Layout::Nudi, &Options::default());
/// assert_eq!(converted.text, "ಕನ್ನಡ");
/// assert!(converted.is_clean());
/// ```
pub fn to_unicode(text: &str, layout: Layout, options: &Options) -> Conversion {
    let table = table(layout);
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut writer = Writer::with_capacity(text.len() * 2);
    let mut i = 0;
    while i < chars.len() {
        let Some((entry, len)) = table.longest_match(chars[i..].iter().map(|&(_, c)| c)) else {
            let starts_syllable = matches!(
                table.longest_match(chars[i + 1..].iter().map(|&(_, c)| c)),
                Some((Entry::Letter(_) | Entry::Base(_), _))
            );
            writer.plain(chars[i], options, starts_syllable);
            i += 1;
            continue;
        };
        writer.apply(entry, &chars[i..i + len]);
        i += len;
    }
    writer.finish()
}

/// Converts text typed in a Nudi or Baraha ANSI font to Unicode Kannada: the same
/// as [`to_unicode`] with [`Layout::Nudi`].
///
/// ```
/// use kannadi::legacy::{nudi_to_unicode, Options};
///
/// assert_eq!(nudi_to_unicode("PÀ£ÀßqÀ", &Options::default()).text, "ಕನ್ನಡ");
/// ```
pub fn nudi_to_unicode(text: &str, options: &Options) -> Conversion {
    to_unicode(text, Layout::Nudi, options)
}

/// Byte offsets in legacy `text` where a syllable or other character starts, in
/// order, always including 0 for text that is not empty. Text split only at
/// these offsets converts piece by piece the same way as whole, apart from a zero
/// typed for ಂ at a split. Signs, subscripts and other marks start a piece only at
/// offset 0.
///
/// ```
/// use kannadi::legacy::{syllable_starts, Layout};
///
/// assert_eq!(syllable_starts("PÀ£ÀßqÀ", Layout::Nudi), [0, 3, 9]);
/// ```
pub fn syllable_starts(text: &str, layout: Layout) -> Vec<usize> {
    let table = table(layout);
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut starts = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let (starts_here, len) = match table.longest_match(chars[i..].iter().map(|&(_, c)| c)) {
            Some((entry, len)) => (
                matches!(entry, Entry::Letter(_) | Entry::Base(_) | Entry::Literal(_)),
                len,
            ),
            None => (true, 1),
        };
        if starts_here || i == 0 {
            starts.push(chars[i].0);
        }
        i += len;
    }
    starts
}

/// [`syllable_starts`] for [`Layout::Nudi`].
pub fn nudi_syllable_starts(text: &str) -> Vec<usize> {
    syllable_starts(text, Layout::Nudi)
}

fn table(layout: Layout) -> &'static Lookup<Entry> {
    static TABLES: [OnceLock<Lookup<Entry>>; Layout::ALL.len()] =
        [OnceLock::new(), OnceLock::new()];
    TABLES[layout.index()].get_or_init(|| {
        let mut lookup = Lookup::new();
        for row in rows(layout) {
            lookup.insert(row.key, row.entry);
        }
        lookup
    })
}

/// Builds the output and both reports in 1 pass.
struct Writer {
    out: String,
    unknown: Vec<Unknown>,
    malformed: Vec<Range<usize>>,
    syllable: Option<Syllable>,
}

impl Writer {
    fn with_capacity(capacity: usize) -> Self {
        Writer {
            out: String::with_capacity(capacity),
            unknown: Vec::new(),
            malformed: Vec::new(),
            syllable: None,
        }
    }

    fn apply(&mut self, entry: &Entry, glyphs: &[(usize, char)]) {
        match entry {
            Entry::Letter(letter) => self.start(letter, false),
            Entry::Base(letter) => self.start(letter, true),
            Entry::Ignore => {}
            Entry::Literal(l) => {
                self.flush();
                self.out.push_str(l);
            }
            _ => match self.syllable.as_mut() {
                Some(s) => {
                    if !s.add(entry) {
                        s.rejected.extend(glyphs.iter().map(|&(_, g)| g));
                        self.report(glyphs);
                    }
                }
                None => self.stray(entry, glyphs),
            },
        }
    }

    fn start(&mut self, letter: &str, needs_sign: bool) {
        self.flush();
        self.syllable = Some(Syllable::new(letter, needs_sign));
    }

    /// Writes a sign, subscript or mark that has no syllable to attach to.
    fn stray(&mut self, entry: &Entry, glyphs: &[(usize, char)]) {
        let start = self.out.len();
        match entry {
            Entry::Sign(sign) if !sign.is_empty() => self.out.push_str(sign),
            Entry::Mod(m) => self.out.push(*m),
            Entry::Virama => self.out.push(VIRAMA),
            Entry::Ottu(c) => {
                self.out.push(VIRAMA);
                self.out.push(*c);
            }
            Entry::Subscripts(cs) => {
                for c in cs.chars() {
                    if class(c) == Class::Consonant {
                        self.out.push(VIRAMA);
                    }
                    self.out.push(c);
                }
            }
            _ => {
                self.report(glyphs);
                self.out.extend(glyphs.iter().map(|&(_, g)| g));
                return;
            }
        }
        self.malformed.push(start..self.out.len());
    }

    /// Writes a character the table does not cover: digits, spaces, punctuation or
    /// an unknown glyph.
    fn plain(&mut self, (offset, c): (usize, char), options: &Options, starts_syllable: bool) {
        self.flush();
        if c.is_ascii_digit() {
            let between_letters =
                starts_syllable && self.out.chars().next_back().is_some_and(is_letter);
            if options.zero_as_anusvara && c == '0' && between_letters {
                self.out.push(ANUSVARA);
            } else if options.kannada_digits {
                self.out.push(to_kannada_digit(c));
            } else {
                self.out.push(c);
            }
            return;
        }
        if !is_passthrough(c) {
            self.unknown.push(Unknown { offset, glyph: c });
        }
        self.out.push(c);
    }

    fn report(&mut self, glyphs: &[(usize, char)]) {
        self.unknown.extend(
            glyphs
                .iter()
                .map(|&(offset, glyph)| Unknown { offset, glyph }),
        );
    }

    fn flush(&mut self) {
        if let Some(s) = self.syllable.take() {
            let start = self.out.len();
            if !s.write(&mut self.out) {
                self.malformed.push(start..self.out.len());
            }
        }
    }

    fn finish(mut self) -> Conversion {
        self.flush();
        Conversion {
            text: self.out,
            unknown: self.unknown,
            malformed: self.malformed,
        }
    }
}

struct Syllable {
    letter: String,
    needs_sign: bool,
    signed: bool,
    signs: String,
    virama: bool,
    ottus: Vec<char>,
    arka: bool,
    mods: String,
    /// Known glyphs that cannot belong here; written after the syllable.
    rejected: String,
}

impl Syllable {
    fn new(letter: &str, needs_sign: bool) -> Self {
        Syllable {
            letter: letter.to_string(),
            needs_sign,
            signed: false,
            signs: String::new(),
            virama: false,
            ottus: Vec::new(),
            arka: false,
            mods: String::new(),
            rejected: String::new(),
        }
    }

    /// Adds a glyph to the syllable; false if it cannot belong here.
    fn add(&mut self, entry: &Entry) -> bool {
        match entry {
            Entry::Virama => {
                self.signed = true;
                !std::mem::replace(&mut self.virama, true)
            }
            Entry::Sign(sign) => {
                self.signed = true;
                self.signs.push_str(sign);
                true
            }
            Entry::Ottu(c) => {
                self.ottus.push(*c);
                true
            }
            Entry::Subscripts(cs) => {
                for c in cs.chars() {
                    if class(c) == Class::Consonant {
                        self.ottus.push(c);
                    } else {
                        self.signed = true;
                        self.signs.push(c);
                    }
                }
                true
            }
            Entry::Arka => !std::mem::replace(&mut self.arka, true),
            Entry::Mod(m) => {
                self.mods.push(*m);
                true
            }
            Entry::Aspirate => self.aspirate(),
            _ => false,
        }
    }

    fn aspirate(&mut self) -> bool {
        let mut chars = self.letter.chars();
        let Some(aspirated) = chars.next().and_then(aspirated) else {
            return false;
        };
        self.letter = std::iter::once(aspirated).chain(chars).collect();
        true
    }

    /// Writes the syllable in Unicode order; false if it is not a valid syllable.
    fn write(&self, out: &mut String) -> bool {
        let mut letter = self.letter.chars();
        let head = letter.next();
        let mut s = String::new();
        if self.arka {
            s.push('ರ');
            s.push(VIRAMA);
        }
        s.extend(head);
        for &o in &self.ottus {
            s.push(VIRAMA);
            s.push(o);
        }
        s.extend(letter);
        s.push_str(&self.signs);
        if self.virama {
            s.push(VIRAMA);
        }
        s.push_str(&self.mods);
        let composed: String = s.nfc().collect();
        let vowel_signs = composed
            .chars()
            .filter(|&c| class(c) == Class::VowelSign)
            .count();
        let on_vowel = head.is_some_and(|c| class(c) == Class::Vowel);
        let valid = !(self.needs_sign && !self.signed)
            && vowel_signs <= 1
            && !(on_vowel && (self.arka || self.signed || !self.ottus.is_empty()));
        out.push_str(&composed);
        out.push_str(&self.rejected);
        valid
    }
}

fn is_letter(c: char) -> bool {
    matches!(class(c), Class::Vowel | Class::Consonant | Class::VowelSign)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy::decode_windows_1252;

    fn conv(legacy: &[u8]) -> Conversion {
        nudi_to_unicode(&decode_windows_1252(legacy), &Options::default())
    }

    fn text(legacy: &[u8]) -> String {
        let c = conv(legacy);
        assert!(c.is_clean(), "{c:?}");
        c.text
    }

    #[test]
    fn head_stroke_gives_the_inherent_vowel() {
        assert_eq!(text(&[0x50, 0xC0]), "ಕ");
        assert_eq!(text(&[0x50, 0xEF]), "ಕ್");
        assert_eq!(text(&[0x50, 0xC1]), "ಕಾ");
        assert_eq!(text(&[0x51]), "ಕಿ");
    }

    #[test]
    fn split_vowel_signs_join() {
        assert_eq!(text(&[0x51, 0xC3]), "ಕೀ");
        assert_eq!(text(&[0x50, 0xC9, 0xC3]), "ಕೇ");
        assert_eq!(text(&[0x50, 0xC9, 0xCA]), "ಕೈ");
        assert_eq!(text(&[0x50, 0xC9, 0xC6]), "ಕೊ");
        assert_eq!(text(&[0x50, 0xC9, 0xC6, 0xC3]), "ಕೋ");
    }

    #[test]
    fn subscripts_move_before_the_vowel_sign() {
        assert_eq!(text(&[0x50, 0xC0, 0xCC]), "ಕ್ಕ");
        assert_eq!(
            text(&[0xB8, 0xC9, 0xD6, 0xE7, 0xF6, 0xCA, 0x6D, 0xEF]),
            "ಸ್ಟ್ರೈಟ್"
        );
        assert_eq!(text(&[0xAE, 0x50, 0xC0, 0xEB, 0xF6, 0xE5]), "ಲಕ್ಷ್ಯ");
    }

    #[test]
    fn arkavattu_moves_to_the_front() {
        assert_eq!(text(&[0x44, 0x67, 0xEF, 0xA9, 0x4C]), "ಆರ್ಬಿಐ");
        assert_eq!(text(&[0x50, 0xC0, 0xF0]), "ರ್ಕ");
    }

    #[test]
    fn composite_consonants() {
        assert_eq!(text(&[0xAA, 0xC0, 0xC4]), "ಮ");
        assert_eq!(text(&[0xAA, 0xC0, 0xC4, 0xC4]), "ಮು");
        assert_eq!(text(&[0xAA, 0xC0, 0xC5]), "ವು");
        assert_eq!(text(&[0xAA, 0xC9, 0xC6]), "ಮೊ");
        assert_eq!(text(&[0xAA, 0xC9, 0xC7]), "ವೊ");
        assert_eq!(text(&[0x41, 0x69, 0xC0, 0xC4]), "ಯ");
        assert_eq!(text(&[0x67, 0xC0, 0x68, 0x69, 0xC1]), "ಝಾ");
    }

    #[test]
    fn mahaprana_mark() {
        assert_eq!(text(&[0xA5, 0x73, 0xC0]), "ಫ");
        assert_eq!(text(&[0xA9, 0xFC]), "ಭಿ");
        assert_eq!(text(&[0x7A, 0x73, 0xC0]), "ಧ");
    }

    #[test]
    fn standalone_vowels_and_marks() {
        assert_eq!(text(&[0x43, 0x41]), "ಅಂ");
        assert_eq!(text(&[0x49, 0xC4]), "ಋ");
        assert_eq!(text(&[0x50, 0xC0, 0x42]), "ಕಃ");
    }

    #[test]
    fn punctuation_from_the_windows_1252_range() {
        assert_eq!(text(&[0x50, 0xC0, 0x86]), "ಕ।");
        assert_eq!(text(&[0x50, 0xC0, 0x87]), "ಕ॥");
        assert_eq!(text(&[0x4A, 0x41, 0x8B, 0x4A, 0x41, 0x8B]), "ಎಂ.ಎಂ.");
    }

    #[test]
    fn zero_between_letters_is_an_anusvara() {
        let prarambha = [0xA5, 0xC0, 0xE6, 0xC1, 0x67, 0xC0, 0x30, 0xA8, 0x73, 0xC0];
        assert_eq!(text(&prarambha), "ಪ್ರಾರಂಭ");
        let off = Options {
            zero_as_anusvara: false,
            ..Options::default()
        };
        let raw = decode_windows_1252(&prarambha);
        assert_eq!(nudi_to_unicode(&raw, &off).text, "ಪ್ರಾರ0ಭ");
        assert_eq!(
            nudi_to_unicode("PÀ 2002", &Options::default()).text,
            "ಕ 2002"
        );
        assert_eq!(nudi_to_unicode("PÀ10", &Options::default()).text, "ಕ10");
    }

    #[test]
    fn kannada_digits_option() {
        let k = Options {
            kannada_digits: true,
            ..Options::default()
        };
        assert_eq!(nudi_to_unicode("PÀ 2026", &k).text, "ಕ ೨೦೨೬");
    }

    #[test]
    fn uncertain_glyphs_stay_unknown() {
        let c = conv(&[0xA5, 0xC0, 0xE6, 0x59, 0xF5, 0xEF]);
        assert_eq!(c.unknown.len(), 1, "{c:?}");
        assert_eq!(c.unknown[0].glyph, '\u{F5}');
    }

    #[test]
    fn unknown_glyphs_are_reported_and_kept() {
        let c = nudi_to_unicode("PÀ\u{F1}", &Options::default());
        assert_eq!(c.text, "ಕ\u{F1}");
        assert_eq!(
            c.unknown,
            [Unknown {
                offset: 3,
                glyph: '\u{F1}'
            }]
        );
    }

    #[test]
    fn stray_signs_are_reported() {
        let c = conv(&[0xC1, 0x50, 0xC0]);
        assert_eq!(c.text, "ಾಕ");
        assert_eq!(c.malformed, vec![0..3]);
    }

    fn report(input: &str) -> Conversion {
        nudi_to_unicode(input, &Options::default())
    }

    #[test]
    fn stray_marks_after_spaces_and_digits_are_malformed() {
        for input in [" Á", "PÀ Á", "1Á", " A", "\nÌ", "†A", " ï"] {
            assert!(!report(input).malformed.is_empty(), "{input:?}");
        }
    }

    #[test]
    fn misplaced_known_glyphs_are_reported() {
        assert_eq!(report("À").unknown.len(), 1);
        assert_eq!(report("PÀðð").unknown.len(), 1);
        assert_eq!(report("Pïï").unknown.len(), 1);
        let c = report("PsÀ");
        assert_eq!(c.unknown.len(), 1);
        assert!(c.text.contains('s'), "{c:?}");
    }

    #[test]
    fn invalid_syllables_are_malformed() {
        for input in ["PÁÁ", "QÁ", "PÉÃÆ", "CÁ", "Dð", "P", "PP"] {
            assert!(!report(input).malformed.is_empty(), "{input:?}");
        }
        assert!(report("PÉÆÃ").is_clean());
    }

    #[test]
    fn syllable_starts_skip_marks() {
        assert_eq!(nudi_syllable_starts(""), Vec::<usize>::new());
        assert_eq!(nudi_syllable_starts("ÀP"), [0, 2]);
        let ksha = decode_windows_1252(&[0x50, 0xC0, 0xEB, 0x20, 0x41, 0x69, 0xC0, 0xC4]);
        assert_eq!(nudi_syllable_starts(&ksha), [0, 5, 6]);
    }

    #[test]
    fn zero_after_an_anusvara_stays_a_digit() {
        assert_eq!(report("PÀA0PÀ").text, "ಕಂ0ಕ");
    }

    fn akruti(legacy: &[u8]) -> Conversion {
        to_unicode(
            &decode_windows_1252(legacy),
            Layout::Akruti,
            &Options::default(),
        )
    }

    #[test]
    fn each_layout_reads_its_own_table() {
        let codes = [0x64, 0xE2];
        assert_eq!(akruti(&codes).text, "ಕ");
        assert_ne!(conv(&codes).text, "ಕ");
    }

    #[test]
    fn a_glyph_can_draw_several_subscripts_and_a_sign() {
        assert_eq!(akruti(&[0xB8, 0xE5, 0xA9, 0xCC, 0xE2, 0xB0]).text, "ಷ್ಟ್ರೀಯ");
        assert_eq!(akruti(&[0xC9, 0xE2, 0xAF, 0xBB]).text, "ಸ್ಕೃತಿ");
        assert_eq!(akruti(&[0x79, 0xEA, 0xE8]).text, "ತ್ತೈ");
        let stray = akruti(&[0x20, 0xE5]);
        assert_eq!(stray.text, " ್ಟ್ರ");
        assert_eq!(stray.malformed.len(), 1);
    }

    #[test]
    fn e_with_the_vocalic_r_glyph_reads_as_ai() {
        assert_eq!(akruti(&[0xC0, 0xEA, 0xBD, 0x70, 0xD2]).text, "ವೈರ್");
    }

    #[test]
    fn a_sign_typed_after_a_space_joins_the_syllable_before() {
        let c = akruti(&[0xC9, 0xE2, 0xAA, 0xC9, 0xE2, 0x22, 0x20, 0xBD, 0xBB]);
        assert!(c.is_clean(), "{c:?}");
        assert_eq!(c.text, "ಸಂಸ್ಕೃತಿ");
        assert_eq!(akruti(&[0x79, 0xEA, 0x20, 0xF4]).text, "ತೈ");
        assert_eq!(
            akruti(&[0xC9, 0xD6, 0x64, 0xE2, 0x5F, 0x20, 0x58]).text,
            "ಸಾಕ್ಷ್ಯ"
        );
        assert_eq!(akruti(&[0x79, 0xE2, 0x20, 0x79, 0xE2]).text, "ತ ತ");
    }
}
