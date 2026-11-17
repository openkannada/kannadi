use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::OnceLock;

use unicode_normalization::UnicodeNormalization;

use crate::akshara::{aksharas, Akshara, AksharaKind};
use crate::chars::{class, Class, VIRAMA};
use crate::numerals::to_ascii_digit;

use super::table::{file_name, rows, Entry, Prefer};
use super::{is_passthrough, unaspirated, windows_1252_byte, Layout};

/// A character [`from_unicode`] could not encode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unencodable {
    /// Byte offset of the character in the input.
    pub offset: usize,
    /// The character.
    pub character: char,
}

/// The result of [`from_unicode`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Encoding {
    /// The glyph codes as Windows-1252 characters, ready to be set in a font with
    /// the layout. Characters that could not be encoded are kept as they were.
    pub text: String,
    /// Characters with no glyph in the layout, in input order. Joiners and nuktas
    /// are kept at the end of their akshara.
    pub unencodable: Vec<Unencodable>,
    /// Byte ranges in the input that are not valid aksharas: a mark with no letter,
    /// 2 vowel signs on 1 consonant, or a sign on a vowel letter. Their glyphs are
    /// written, but they may not read back as the same text.
    pub malformed: Vec<Range<usize>>,
}

impl Encoding {
    /// True if every character was encoded and every akshara is well formed.
    pub fn is_clean(&self) -> bool {
        self.unencodable.is_empty() && self.malformed.is_empty()
    }
}

/// Converts Unicode Kannada to the glyph codes of the fonts with layout `layout`.
///
/// Each akshara is written in the order typists stored it: the consonant shape,
/// the vowel sign, the subscript consonants, ೃ and the length marks ೕ ೖ, the
/// arkavattu and then ಂ or ಃ. Where the layout has 2 glyphs for 1 sign, the one the
/// font uses in that position is chosen.
///
/// When the result [`is_clean`](Encoding::is_clean),
/// [`to_unicode`](super::to_unicode) gives back the NFC form of the input.
/// Kannada digits are written with the layout's glyphs for them, and a digit the
/// layout has no glyph for, such as ೦ in Akruti, is reported. A layout with no
/// Kannada digit glyphs, such as Nudi, gets ASCII digits, which the `k` variants of the Nudi fonts
/// draw as Kannada digits. A digit zero written between 2 letters reads back as ಂ
/// unless [`Options::zero_as_anusvara`](super::Options::zero_as_anusvara) is off.
///
/// ```
/// use kannadi::legacy::{from_unicode, Layout};
///
/// let encoded = from_unicode("ಕನ್ನಡ", Layout::Nudi);
/// assert_eq!(encoded.text, "PÀ£ÀßqÀ");
/// assert!(encoded.is_clean());
///
/// let mixed = from_unicode("ಕ A", Layout::Nudi);
/// assert_eq!(mixed.unencodable[0].character, 'A');
/// ```
pub fn from_unicode(text: &str, layout: Layout) -> Encoding {
    let mut writer = Writer {
        glyphs: glyphs(layout),
        text,
        out: String::with_capacity(text.len()),
        unencodable: Vec::new(),
    };
    let mut malformed = Vec::new();
    let mut offset = 0;
    for akshara in aksharas(text) {
        let chars: Vec<(usize, char)> = akshara
            .text
            .char_indices()
            .map(|(i, c)| (offset + i, c))
            .collect();
        if !is_well_formed(&akshara) {
            malformed.push(offset..offset + akshara.text.len());
        }
        match Parts::of(&chars) {
            Some(parts) => {
                for part in parts.split_at_missing_subscript(writer.glyphs) {
                    writer.consonant(&part);
                }
            }
            None => chars.iter().for_each(|&c| writer.single(c)),
        }
        offset += akshara.text.len();
    }
    Encoding {
        text: writer.out,
        unencodable: writer.unencodable,
        malformed,
    }
}

/// Converts Unicode Kannada to the glyph codes of the Nudi and Baraha ANSI fonts:
/// the same as [`from_unicode`] with [`Layout::Nudi`].
///
/// ```
/// use kannadi::legacy::unicode_to_nudi;
///
/// assert_eq!(unicode_to_nudi("ಕನ್ನಡ").text, "PÀ£ÀßqÀ");
/// ```
pub fn unicode_to_nudi(text: &str) -> Encoding {
    from_unicode(text, Layout::Nudi)
}

fn is_well_formed(akshara: &Akshara) -> bool {
    let mut classes = akshara.text.chars().map(class);
    match akshara.kind {
        AksharaKind::Other => !classes.any(Class::is_mark),
        AksharaKind::Vowel => !classes.any(|c| matches!(c, Class::VowelSign | Class::Virama)),
        AksharaKind::Consonant => {
            let composed = akshara.text.nfc().map(class);
            composed.filter(|&c| c == Class::VowelSign).count() <= 1
        }
    }
}

/// Vowel signs the layouts draw on the letter form of a consonant. The other signs
/// take the place of the head stroke, so they go on the base form.
const LETTER_SIGNS: [char; 3] = ['ಿ', 'ು', 'ೂ'];

/// Signs typists stored after the subscript consonants: ೃ (U+0CC3) and the length
/// marks ೕ (U+0CD5) and ೖ (U+0CD6).
const AFTER_SUBSCRIPTS: [char; 3] = ['ೃ', '\u{0CD5}', '\u{0CD6}'];

const RA: char = 'ರ';

struct Choice {
    key: String,
    prefer: Prefer,
}

/// The glyph table indexed by meaning.
#[derive(Default)]
struct Glyphs {
    /// Keyed by the NFD form of the letter.
    letters: HashMap<String, String>,
    bases: HashMap<char, String>,
    /// Keyed by vowel sign part, and the virama.
    signs: HashMap<char, Vec<Choice>>,
    head_stroke: String,
    ottus: HashMap<char, Vec<Choice>>,
    arka: Vec<Choice>,
    aspirate: Vec<Choice>,
    mods: HashMap<char, String>,
    literals: HashMap<char, String>,
    /// The layout has glyphs for Kannada digits, so ASCII digits would show as
    /// Latin digits.
    kannada_digits: bool,
    /// First characters of all glyph codes; anything else passes through decoding.
    key_starts: HashSet<char>,
}

fn glyphs(layout: Layout) -> &'static Glyphs {
    static GLYPHS: OnceLock<Vec<Glyphs>> = OnceLock::new();
    let all = GLYPHS.get_or_init(|| Layout::ALL.iter().map(|&l| build(l)).collect());
    &all[layout.index()]
}

/// Indexes a layout's table by meaning.
fn build(layout: Layout) -> Glyphs {
    let file = file_name(layout);
    let mut g = Glyphs::default();
    for row in rows(layout) {
        if row.prefer == Prefer::Read {
            continue;
        }
        g.key_starts.extend(row.key.chars().next());
        let choice = Choice {
            key: row.key,
            prefer: row.prefer,
        };
        let choosable = matches!(
            row.entry,
            Entry::Sign(ref s) if !s.is_empty()
        ) || matches!(
            row.entry,
            Entry::Virama | Entry::Ottu(..) | Entry::Arka | Entry::Aspirate
        );
        assert!(
            choosable || choice.prefer == Prefer::Otherwise,
            "{file}: a preference on {:?} has no effect",
            row.entry
        );
        match row.entry {
            Entry::Letter(l) => {
                g.letters.entry(l.nfd().collect()).or_insert(choice.key);
            }
            Entry::Base(b) => {
                let c = single(&b, file);
                g.bases.entry(c).or_insert(choice.key);
            }
            Entry::Sign(s) if s.is_empty() => {
                if g.head_stroke.is_empty() {
                    g.head_stroke = choice.key;
                }
            }
            Entry::Sign(s) => g.signs.entry(single(&s, file)).or_default().push(choice),
            Entry::Virama => g.signs.entry(VIRAMA).or_default().push(choice),
            Entry::Ottu(consonants, None) if consonants.len() == 1 => {
                g.ottus.entry(consonants[0]).or_default().push(choice);
            }
            Entry::Arka => g.arka.push(choice),
            Entry::Aspirate => g.aspirate.push(choice),
            Entry::Mod(m) => {
                g.mods.entry(m).or_insert(choice.key);
            }
            Entry::Literal(l) => {
                let c = single(&l, file);
                g.kannada_digits |= class(c) == Class::Digit && !c.is_ascii();
                g.literals.entry(c).or_insert(choice.key);
            }
            Entry::Ottu(..) | Entry::Ignore => {}
        }
    }
    g
}

fn single(value: &str, file: &str) -> char {
    let mut chars = value.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => c,
        _ => panic!("{file}: expected 1 character, found {value:?}"),
    }
}

fn choose_sign<'a>(glyphs: &'a Glyphs, part: char, context: &Context) -> Option<&'a str> {
    glyphs.signs.get(&part).and_then(|s| choose(s, context))
}

/// What a glyph's preference is checked against.
struct Context<'a> {
    consonant: Option<char>,
    signs: &'a [char],
    after_ottu: bool,
}

/// The glyph to write: the first whose preference holds, or else the first with none.
fn choose<'a>(choices: &'a [Choice], context: &Context) -> Option<&'a str> {
    let holds = |p: &Prefer| match p {
        Prefer::Otherwise | Prefer::Read => false,
        Prefer::On(cs) => context.consonant.is_some_and(|c| cs.contains(c)),
        Prefer::With(cs) => context.signs.iter().any(|&s| cs.contains(s)),
        Prefer::AfterOttu => context.after_ottu,
    };
    choices
        .iter()
        .find(|c| holds(&c.prefer))
        .or_else(|| choices.iter().find(|c| c.prefer == Prefer::Otherwise))
        .map(|c| c.key.as_str())
}

/// A consonant akshara taken apart, each character with its byte offset. There is
/// always at least 1 consonant.
#[derive(Default)]
struct Parts {
    consonants: Vec<(usize, char)>,
    /// Vowel sign parts in NFD order, then a final virama.
    signs: Vec<(usize, char)>,
    mods: Vec<(usize, char)>,
    /// Joiners and nuktas, which the layout has no glyph for.
    other: Vec<(usize, char)>,
}

impl Parts {
    /// None unless `chars` starts with a consonant.
    fn of(chars: &[(usize, char)]) -> Option<Self> {
        if chars.first().map(|&(_, c)| class(c)) != Some(Class::Consonant) {
            return None;
        }
        let mut parts = Parts::default();
        for (n, &(i, c)) in chars.iter().enumerate() {
            let class_at = |k: usize| chars.get(k).map(|&(_, c)| class(c));
            match class(c) {
                Class::Consonant => parts.consonants.push((i, c)),
                Class::Virama => {
                    let joins = match class_at(n + 1) {
                        Some(Class::Consonant) => true,
                        Some(Class::Zwj) => class_at(n + 2) == Some(Class::Consonant),
                        _ => false,
                    };
                    if !joins {
                        parts.signs.push((i, c));
                    }
                }
                Class::VowelSign => parts.signs.extend(c.nfd().map(|p| (i, p))),
                Class::Modifier => parts.mods.push((i, c)),
                _ => parts.other.push((i, c)),
            }
        }
        Some(parts)
    }
}

impl Parts {
    /// The akshara as written: whole, or, where the layout has no glyph for a
    /// subscript consonant, split before it, the part before ending in a visible
    /// virama. Both read back as the same Unicode.
    fn split_at_missing_subscript(self, glyphs: &Glyphs) -> Vec<Parts> {
        let arka = usize::from(self.consonants.len() > 1 && self.consonants[0].1 == RA);
        let missing = self.consonants[arka..]
            .iter()
            .skip(1)
            .position(|(_, c)| !glyphs.ottus.contains_key(c));
        let Some(j) = missing.filter(|_| glyphs.signs.contains_key(&VIRAMA)) else {
            return vec![self];
        };
        let at = arka + 1 + j;
        let mut rest = self;
        let first = Parts {
            consonants: rest.consonants.drain(..at).collect(),
            signs: vec![(rest.consonants[0].0, VIRAMA)],
            ..Parts::default()
        };
        let mut parts = vec![first];
        parts.extend(rest.split_at_missing_subscript(glyphs));
        parts
    }
}

struct Writer<'a> {
    glyphs: &'static Glyphs,
    /// The input, to report a vowel sign whole when only some of its parts have glyphs.
    text: &'a str,
    out: String,
    unencodable: Vec<Unencodable>,
}

impl Writer<'_> {
    fn consonant(&mut self, parts: &Parts) {
        let arka = parts.consonants.len() > 1 && parts.consonants[0].1 == RA;
        let cluster = &parts.consonants[usize::from(arka)..];
        let (head_offset, head) = cluster[0];
        let sign_chars: Vec<char> = parts.signs.iter().map(|&(_, c)| c).collect();
        let (before, after): (Vec<_>, Vec<_>) = parts
            .signs
            .iter()
            .partition(|&&(_, c)| !AFTER_SUBSCRIPTS.contains(&c));
        let mut context = Context {
            consonant: Some(head),
            signs: &sign_chars,
            after_ottu: false,
        };

        let before_chars: Vec<char> = before.iter().map(|&(_, c)| c).collect();
        let used = match self.head(head, &before_chars, &context) {
            Some((glyphs, used)) => {
                self.out.push_str(&glyphs);
                used
            }
            None => {
                self.unencodable(head_offset, head);
                0
            }
        };
        let unwritable: Vec<usize> = before[used..]
            .iter()
            .chain(&after)
            .filter(|&&(_, c)| !self.glyphs.signs.contains_key(&c))
            .map(|&(i, _)| i)
            .collect();
        let mut reported = Vec::new();
        let mut sign = |w: &mut Self, (i, c): (usize, char), context: &Context| {
            if !unwritable.contains(&i) {
                w.write(choose_sign(w.glyphs, c, context), i, c);
            } else if !reported.contains(&i) {
                reported.push(i);
                let whole = w.text[i..].chars().next().unwrap_or(c);
                w.unencodable(i, whole);
            }
        };
        for &(i, c) in &before[used..] {
            sign(self, (i, c), &context);
        }
        for &(i, c) in &cluster[1..] {
            let key = self.glyphs.ottus.get(&c).and_then(|o| choose(o, &context));
            self.write(key, i, c);
            context.after_ottu = true;
        }
        for &(i, c) in &after {
            sign(self, (i, c), &context);
            context.after_ottu = false;
        }
        if arka {
            let (i, c) = parts.consonants[0];
            self.write(choose(&self.glyphs.arka, &context), i, c);
        }
        for &(i, m) in &parts.mods {
            self.write(self.glyphs.mods.get(&m).map(String::as_str), i, m);
        }
        for &(i, c) in &parts.other {
            self.unencodable(i, c);
        }
    }

    /// The glyphs for the consonant shape, and how many of `signs` they include.
    fn head(&self, consonant: char, signs: &[char], context: &Context) -> Option<(String, usize)> {
        let g = self.glyphs;
        let letter = |c: char, signs: &[char]| {
            let mut key = String::from(c);
            key.extend(signs);
            g.letters.get(&key)
        };
        let plain = unaspirated(consonant);
        let aspirate = || choose(&g.aspirate, context).unwrap_or_default();
        for n in (1..=signs.len()).rev() {
            if let Some(key) = letter(consonant, &signs[..n]) {
                return Some((key.clone(), n));
            }
            if let Some(key) = plain.and_then(|p| letter(p, &signs[..n])) {
                return Some((format!("{key}{}", aspirate()), n));
            }
        }
        let on_letter = signs.first().map_or(true, |s| LETTER_SIGNS.contains(s));
        if on_letter {
            if let Some(key) = letter(consonant, &[]) {
                return Some((key.clone(), 0));
            }
        }
        let base = match (g.bases.get(&consonant), plain) {
            (Some(key), _) => Some(key.clone()),
            (None, Some(p)) => g.bases.get(&p).map(|key| format!("{key}{}", aspirate())),
            (None, None) => None,
        };
        match base {
            Some(mut key) => {
                if on_letter {
                    key.push_str(&g.head_stroke);
                }
                Some((key, 0))
            }
            None => letter(consonant, &[]).map(|key| (key.clone(), 0)),
        }
    }

    /// Writes a character outside a consonant akshara.
    fn single(&mut self, (offset, c): (usize, char)) {
        let g = self.glyphs;
        let plain = c.is_ascii_digit() || is_passthrough(c);
        // Outside ASCII, a layout may draw punctuation at other codes, as Akruti
        // draws – at F6 and leaves 96 empty.
        let drawn_as_itself = c.is_ascii() || !g.literals.contains_key(&c);
        if plain && drawn_as_itself && windows_1252_byte(c).is_some() && !g.key_starts.contains(&c)
        {
            self.out.push(c);
            return;
        }
        let context = Context {
            consonant: None,
            signs: &[],
            after_ottu: false,
        };
        match class(c) {
            Class::Digit => match g.literals.get(&c) {
                Some(key) => self.out.push_str(key),
                None if g.kannada_digits => self.unencodable(offset, c),
                None => self.out.push(to_ascii_digit(c)),
            },
            Class::Vowel => {
                let key = g.letters.get(&c.nfd().collect::<String>());
                self.write(key.map(String::as_str), offset, c);
            }
            Class::VowelSign | Class::Virama => {
                let keys: Option<String> = c
                    .nfd()
                    .map(|part| g.signs.get(&part).and_then(|s| choose(s, &context)))
                    .collect();
                self.write(keys.as_deref(), offset, c);
            }
            Class::Modifier => self.write(g.mods.get(&c).map(String::as_str), offset, c),
            _ => self.write(g.literals.get(&c).map(String::as_str), offset, c),
        }
    }

    fn write(&mut self, key: Option<&str>, offset: usize, c: char) {
        match key {
            Some(key) => self.out.push_str(key),
            None => self.unencodable(offset, c),
        }
    }

    fn unencodable(&mut self, offset: usize, character: char) {
        self.out.push(character);
        self.unencodable.push(Unencodable { offset, character });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy::{nudi_to_unicode, to_unicode, Options};

    fn encode(text: &str) -> Vec<u8> {
        let e = unicode_to_nudi(text);
        assert!(e.is_clean(), "{e:?}");
        e.text
            .chars()
            .map(|c| windows_1252_byte(c).expect("Windows-1252 character"))
            .collect()
    }

    fn roundtrip(text: &str) {
        let back = nudi_to_unicode(&unicode_to_nudi(text).text, &Options::default());
        assert!(back.is_clean(), "{back:?}");
        assert_eq!(back.text, text);
    }

    #[test]
    fn inherent_vowel_takes_the_head_stroke() {
        assert_eq!(encode("ಕ"), [0x50, 0xC0]);
        assert_eq!(encode("ಕನ್ನಡ"), [0x50, 0xC0, 0xA3, 0xC0, 0xDF, 0x71, 0xC0]);
    }

    #[test]
    fn letter_forms_carry_i_u_and_uu() {
        assert_eq!(encode("ಕಿ"), [0x51]);
        assert_eq!(encode("ಜ"), [0x64]);
        assert_eq!(encode("ಜು"), [0x64, 0xC4]);
        assert_eq!(encode("ಣಿ"), [0x74, 0xC2]);
        assert_eq!(encode("ಸು"), [0xB8, 0xC0, 0xC4]);
    }

    #[test]
    fn other_signs_go_on_the_base_form() {
        assert_eq!(encode("ಜೆ"), [0x65, 0xC9]);
        assert_eq!(encode("ಲಾ"), [0xAF, 0xC1]);
        assert_eq!(encode("ಟ್"), [0x6D, 0xEF]);
        assert_eq!(encode("ಪೌ"), [0xA5, 0xCB]);
    }

    #[test]
    fn split_vowel_signs() {
        assert_eq!(encode("ಕೀ"), [0x51, 0xC3]);
        assert_eq!(encode("ಕೇ"), [0x50, 0xC9, 0xC3]);
        assert_eq!(encode("ಕೈ"), [0x50, 0xC9, 0xCA]);
        assert_eq!(encode("ಕೊ"), [0x50, 0xC9, 0xC6]);
        assert_eq!(encode("ಕೋ"), [0x50, 0xC9, 0xC6, 0xC3]);
    }

    #[test]
    fn subscripts_follow_the_sign_and_length_marks_follow_them() {
        assert_eq!(encode("ತ್ತು"), [0x76, 0xC0, 0xC4, 0xDB]);
        assert_eq!(encode("ದ್ದೇ"), [0x7A, 0xC9, 0xDD, 0xC3]);
        assert_eq!(encode("ಕ್ರೀ"), [0x51, 0xE6, 0xC3]);
        assert_eq!(encode("ಸ್ಟ್ರೈ"), [0xB8, 0xC9, 0xD6, 0xE7, 0xCA]);
        assert_eq!(encode("ಷ್ಕೃ"), [0xB5, 0xC0, 0xCC, 0xF8]);
        assert_eq!(encode("ಕೃ"), [0x50, 0xC0, 0xC8]);
        assert_eq!(encode("ಚ್ಚ್"), [0x5A, 0xEF, 0xD1]);
    }

    #[test]
    fn second_ra_subscript_uses_its_own_glyph() {
        assert_eq!(encode("ಪ್ರ"), [0xA5, 0xC0, 0xE6]);
        assert_eq!(encode("ಸ್ತ್ರ"), [0xB8, 0xC0, 0xDB, 0xE7]);
    }

    #[test]
    fn arkavattu_comes_after_the_cluster() {
        assert_eq!(encode("ರ್ಕ"), [0x50, 0xC0, 0xF0]);
        assert_eq!(encode("ರ್ದೇ"), [0x7A, 0xC9, 0xC3, 0xF0]);
        assert_eq!(encode("ರ್ಥ್ಯ"), [0x78, 0xC0, 0xE5, 0xF0]);
        assert_eq!(encode("ರ್ಕಂ"), [0x50, 0xC0, 0xF0, 0x41]);
    }

    #[test]
    fn composite_consonants() {
        assert_eq!(encode("ಮ"), [0xAA, 0xC0, 0xC4]);
        assert_eq!(encode("ಮು"), [0xAA, 0xC0, 0xC4, 0xC4]);
        assert_eq!(encode("ಮಾ"), [0xAA, 0xC0, 0x69, 0xC1]);
        assert_eq!(encode("ಮೀ"), [0xAB, 0xC4, 0xC3]);
        assert_eq!(encode("ಯೇ"), [0x41, 0x69, 0xC9, 0xC4, 0xC3]);
        assert_eq!(encode("ಯೋ"), [0x41, 0x69, 0xC9, 0xC6, 0xC3]);
        assert_eq!(encode("ಝಾ"), [0x67, 0xC0, 0x68, 0x69, 0xC1]);
    }

    #[test]
    fn pa_and_va_take_their_own_u_signs() {
        assert_eq!(encode("ವು"), [0xAA, 0xC0, 0xC5]);
        assert_eq!(encode("ಪೂ"), [0xA5, 0xC0, 0xC7]);
        assert_eq!(encode("ವೊ"), [0xAA, 0xC9, 0xC7]);
        assert_eq!(encode("ಗು"), [0x55, 0xC0, 0xC4]);
    }

    #[test]
    fn aspirated_consonants() {
        assert_eq!(encode("ಭ"), [0xA8, 0x73, 0xC0]);
        assert_eq!(encode("ಭೂ"), [0xA8, 0x73, 0xC0, 0xC6]);
        assert_eq!(encode("ಧಾ"), [0x7A, 0x73, 0xC1]);
        assert_eq!(encode("ಧಿ"), [0xA2, 0xFC]);
        assert_eq!(encode("ಢೀ"), [0x72, 0xFC, 0xC3]);
    }

    #[test]
    fn vowels_marks_digits_and_punctuation() {
        assert_eq!(encode("ಅಂ"), [0x43, 0x41]);
        assert_eq!(encode("ಋ"), [0x49, 0xC4]);
        assert_eq!(encode("ಕಃ"), [0x50, 0xC0, 0x42]);
        assert_eq!(encode("ಕ।"), [0x50, 0xC0, 0x86]);
        assert_eq!(encode("೨೦೨೬, ಕ."), b"2026, P\xC0.");
    }

    #[test]
    fn unencodable_characters_are_reported_and_kept() {
        let e = unicode_to_nudi("ಕ A ಌ ಕ಼");
        let found: Vec<(usize, char)> = e
            .unencodable
            .iter()
            .map(|u| (u.offset, u.character))
            .collect();
        assert_eq!(found, [(4, 'A'), (6, 'ಌ'), (13, '\u{0CBC}')]);
        assert!(e.text.contains('A') && e.text.contains('ಌ'));
        assert!(!unicode_to_nudi("„").is_clean());
    }

    #[test]
    fn zwnj_is_reported_and_its_virama_kept() {
        let e = unicode_to_nudi("ಡ್\u{200C}ಗೆ");
        assert_eq!(e.unencodable.len(), 1);
        let back = nudi_to_unicode(&e.text.replace('\u{200C}', ""), &Options::default());
        assert_eq!(back.text, "ಡ್ಗೆ");
    }

    #[test]
    fn zwj_is_reported_and_the_cluster_kept() {
        let e = unicode_to_nudi("ರ್\u{200D}ಕ");
        assert_eq!(e.unencodable.len(), 1);
        let back = nudi_to_unicode(&e.text.replace('\u{200D}', ""), &Options::default());
        assert_eq!(back.text, "ರ್ಕ");
        let e = unicode_to_nudi("ಕ್\u{200D}");
        assert_eq!(e.unencodable.len(), 1);
        let back = nudi_to_unicode(&e.text.replace('\u{200D}', ""), &Options::default());
        assert_eq!(back.text, "ಕ್");
    }

    #[test]
    fn nakaara_pollu_is_reported() {
        let e = unicode_to_nudi("ವೝು");
        assert_eq!(e.unencodable[0].character, 'ೝ');
        assert!(e.text.contains('ೝ'));
    }

    #[test]
    fn malformed_aksharas_are_reported() {
        for text in ["ಕಂಾ", "ಕೇೂ", "ಠೕೆ", "ಅಾ", "ಾ", "1ಾ", " ್"] {
            let e = unicode_to_nudi(text);
            assert!(!e.malformed.is_empty(), "{text}");
        }
        let e = unicode_to_nudi("ಕ ಕೇೂ");
        assert_eq!(e.malformed, vec![4..13]);
        assert!(unicode_to_nudi("ಕಿ\u{0CD5}").is_clean());
    }

    #[test]
    fn characters_without_a_windows_1252_byte_are_reported() {
        for text in ["ಕ\u{2003}ಕ", "ಕ\u{2010}", "\u{3000}"] {
            assert!(!unicode_to_nudi(text).unencodable.is_empty(), "{text:?}");
        }
        assert!(unicode_to_nudi("ಕ\u{A0}ಕ\u{2013}").is_clean());
    }

    #[test]
    fn government_page_words_roundtrip() {
        for word in [
            "ಪ್ರಾರಂಭ",
            "ಸ್ಟ್ರೈಟ್",
            "ಲಕ್ಷ್ಯ",
            "ಆರ್ಬಿಐ",
            "ಕಾರ್ಯಕ್ರಮ",
            "ಪರಿಷ್ಕೃತ",
            "ಜ್ಞಾನ",
        ] {
            roundtrip(word);
        }
    }

    fn akruti(text: &str) -> Vec<u8> {
        let e = from_unicode(text, Layout::Akruti);
        assert!(e.is_clean(), "{e:?}");
        e.text
            .chars()
            .map(|c| windows_1252_byte(c).expect("Windows-1252 character"))
            .collect()
    }

    #[test]
    fn each_layout_writes_its_own_glyphs() {
        assert_eq!(akruti("ಕನ್ನಡ"), [0x64, 0xE2, 0x7D, 0xE2, 0x51, 0x76, 0xE2]);
        assert_eq!(akruti("ಮುಖ"), [0xC0, 0xE2, 0xB0, 0xB0, 0xD9]);
    }

    #[test]
    fn spaces_are_written_as_spaces_where_a_sign_may_follow_one() {
        assert_eq!(akruti("ಕ ಕ"), [0x64, 0xE2, 0x20, 0x64, 0xE2]);
        assert_eq!(akruti("ಕ\u{A0}ಕ"), [0x64, 0xE2, 0xA0, 0x64, 0xE2]);
    }

    #[test]
    fn punctuation_outside_ascii_uses_the_layout_glyph() {
        assert_eq!(akruti("‘ಕ’"), [0x60, 0x64, 0xE2, 0x27]);
        assert_eq!(akruti("ಕ–ಕ"), [0x64, 0xE2, 0xF6, 0x64, 0xE2]);
        assert_eq!(encode("ಕ. ಕ।"), [0x50, 0xC0, 0x2E, 0x20, 0x50, 0xC0, 0x86]);
    }

    #[test]
    fn akruti_words_roundtrip() {
        for word in ["ಸರ್ಕಾರ", "ಮೋಟಾರು", "ಯೋಜನೆ", "ಶ್ರೀ", "ಋಣ", "ಸಾಮಾಜಿಕ", "ವಿಶ್ವವಿದ್ಯಾನಿಲಯ"]
        {
            let back = to_unicode(
                &from_unicode(word, Layout::Akruti).text,
                Layout::Akruti,
                &Options::default(),
            );
            assert!(back.is_clean(), "{back:?}");
            assert_eq!(back.text, word);
        }
    }

    #[test]
    fn a_sign_with_a_part_the_layout_lacks_is_reported_whole() {
        let e = from_unicode("ಞೀ", Layout::Akruti);
        assert_eq!(
            e.unencodable,
            [Unencodable {
                offset: 3,
                character: 'ೀ'
            }]
        );
        assert_eq!(e.text, "qೀ");
    }

    fn shree(text: &str) -> Vec<u8> {
        let e = from_unicode(text, Layout::ShreeLipi);
        assert!(e.is_clean(), "{e:?}");
        e.text
            .chars()
            .map(|c| windows_1252_byte(c).expect("Windows-1252 character"))
            .collect()
    }

    #[test]
    fn shree_lipi_writes_as_typists_did() {
        assert_eq!(shree("ಕನ್ನಡ"), [0x50, 0xDC, 0xAE, 0xDC, 0xB0, 0x76, 0xDC]);
        assert_eq!(shree("ಮಾಮು"), [0xCA, 0xDC, 0xDE, 0xCA, 0xDC, 0xE1, 0xE1]);
        assert_eq!(shree("ಲಕ್ಷ"), [0xC6, 0x50, 0xDC, 0xD2]);
        assert_eq!(shree("ಕ-ಕ"), [0x50, 0xDC, 0x26, 0x50, 0xDC]);
        assert_eq!(shree("ಪುವೂ"), [0xB1, 0xDC, 0xE2, 0xCA, 0xDC, 0xE4]);
        assert_eq!(shree("ತ್ರ್ಯ"), [0xF1, 0xDC, 0xC5, 0xE9]);
        assert_eq!(shree("ಝೆ"), [0xC3, 0xE6, 0x6B, 0xE1]);
        let dollar = from_unicode("$", Layout::ShreeLipi);
        assert_eq!(dollar.unencodable.len(), 1);
    }

    #[test]
    fn a_subscript_the_layout_cannot_draw_is_written_after_a_virama() {
        let e = from_unicode("ಕ್ಶ", Layout::NudiB);
        assert!(e.is_clean(), "{e:?}");
        let bytes: Vec<u8> = e.text.chars().filter_map(windows_1252_byte).collect();
        assert_eq!(bytes, [0x91, 0xFD, 0xC6, 0xD0]);
        assert_eq!(encode("ಕ್ಗ"), [0x50, 0xC0, 0xCE]);
    }

    #[test]
    fn kannada_digits_use_the_layout_glyphs() {
        assert_eq!(akruti("೧೨೯"), [0x81, 0x82, 0x89]);
        let zero = from_unicode("೧೦", Layout::Akruti);
        assert_eq!(zero.unencodable.len(), 1);
        assert_eq!(zero.unencodable[0].character, '೦');
        assert_eq!(encode("೧೦"), [0x31, 0x30]);
    }
}
