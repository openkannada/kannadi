use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use unicode_normalization::UnicodeNormalization;

use crate::akshara::{aksharas, AksharaKind};
use crate::chars::{class, Class, VIRAMA};
use crate::numerals::to_ascii_digit;

use super::table::{rows, Entry, Prefer};
use super::{is_passthrough, unaspirated};

/// A character [`unicode_to_nudi`] could not encode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Unencodable {
    /// Byte offset of the character in the input.
    pub offset: usize,
    /// The character.
    pub character: char,
}

/// The result of [`unicode_to_nudi`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Encoding {
    /// The glyph codes as Windows-1252 characters, ready to be set in a Nudi or
    /// Baraha ANSI font. Characters that could not be encoded are kept as they were.
    pub text: String,
    /// Characters with no glyph in the layout, in input order.
    pub unencodable: Vec<Unencodable>,
}

impl Encoding {
    /// True if every character was encoded.
    pub fn is_complete(&self) -> bool {
        self.unencodable.is_empty()
    }
}

/// Converts Unicode Kannada to the glyph codes of the Nudi and Baraha ANSI fonts.
///
/// Each akshara is written in the order typists stored it: the consonant shape,
/// the vowel sign, the subscript consonants, the length marks, the arkavattu and
/// then ಂ or ಃ. Where the layout has 2 glyphs for 1 sign, the one the font uses in
/// that position is chosen.
///
/// For well-formed Kannada with no unencodable characters,
/// [`nudi_to_unicode`](super::nudi_to_unicode) gives back the NFC form of the input.
/// Kannada digits are written as ASCII digits, which the `k` variants of the fonts
/// draw as Kannada digits. A digit zero written between 2 letters reads back as ಂ
/// unless [`Options::zero_as_anusvara`](super::Options::zero_as_anusvara) is off.
///
/// ```
/// use kannadi::legacy::unicode_to_nudi;
///
/// let encoded = unicode_to_nudi("ಕನ್ನಡ");
/// assert_eq!(encoded.text, "PÀ£ÀßqÀ");
/// assert!(encoded.is_complete());
///
/// let mixed = unicode_to_nudi("ಕ A");
/// assert_eq!(mixed.unencodable[0].character, 'A');
/// ```
pub fn unicode_to_nudi(text: &str) -> Encoding {
    let mut writer = Writer {
        glyphs: glyphs(),
        out: String::with_capacity(text.len()),
        unencodable: Vec::new(),
    };
    let mut offset = 0;
    for akshara in aksharas(text) {
        let chars = akshara.text.char_indices().map(|(i, c)| (offset + i, c));
        match akshara.kind {
            AksharaKind::Consonant => writer.consonant(&Parts::of(chars)),
            AksharaKind::Vowel | AksharaKind::Other => chars.for_each(|c| writer.single(c)),
        }
        offset += akshara.text.len();
    }
    Encoding {
        text: writer.out,
        unencodable: writer.unencodable,
    }
}

/// Vowel signs Nudi draws on the letter form of a consonant. The other signs take
/// the place of the head stroke, so they go on the base form.
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
    /// First characters of all glyph codes; anything else passes through decoding.
    key_starts: HashSet<char>,
}

fn glyphs() -> &'static Glyphs {
    static GLYPHS: OnceLock<Glyphs> = OnceLock::new();
    GLYPHS.get_or_init(|| {
        let mut g = Glyphs::default();
        for row in rows() {
            g.key_starts.extend(row.key.chars().next());
            let choice = Choice {
                key: row.key,
                prefer: row.prefer,
            };
            match row.entry {
                Entry::Letter(l) => {
                    g.letters.entry(l.nfd().collect()).or_insert(choice.key);
                }
                Entry::Base(b) => {
                    let c = single(&b);
                    g.bases.entry(c).or_insert(choice.key);
                }
                Entry::Sign(s) if s.is_empty() => {
                    if g.head_stroke.is_empty() {
                        g.head_stroke = choice.key;
                    }
                }
                Entry::Sign(s) => g.signs.entry(single(&s)).or_default().push(choice),
                Entry::Virama => g.signs.entry(VIRAMA).or_default().push(choice),
                Entry::Ottu(c) => g.ottus.entry(c).or_default().push(choice),
                Entry::Arka => g.arka.push(choice),
                Entry::Aspirate => g.aspirate.push(choice),
                Entry::Mod(m) => {
                    g.mods.entry(m).or_insert(choice.key);
                }
                Entry::Literal(l) => {
                    g.literals.entry(single(&l)).or_insert(choice.key);
                }
                Entry::Ignore => {}
            }
        }
        g
    })
}

fn single(value: &str) -> char {
    let mut chars = value.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => c,
        _ => panic!("expected 1 character, found {value:?} in nudi.tsv"),
    }
}

/// What a glyph's preference is checked against.
struct Context<'a> {
    consonant: char,
    signs: &'a [char],
    after_ottu: bool,
}

/// The glyph to write: the first whose preference holds, or else the first with none.
fn choose<'a>(choices: &'a [Choice], context: &Context) -> Option<&'a str> {
    let holds = |p: &Prefer| match p {
        Prefer::Always => false,
        Prefer::On(cs) => cs.contains(context.consonant),
        Prefer::With(cs) => context.signs.iter().any(|&s| cs.contains(s)),
        Prefer::AfterOttu => context.after_ottu,
    };
    choices
        .iter()
        .find(|c| holds(&c.prefer))
        .or_else(|| choices.iter().find(|c| c.prefer == Prefer::Always))
        .map(|c| c.key.as_str())
}

/// A consonant akshara taken apart, each character with its byte offset.
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
    fn of(chars: impl Iterator<Item = (usize, char)>) -> Self {
        let mut parts = Parts::default();
        let mut chars = chars.peekable();
        while let Some((i, c)) = chars.next() {
            match class(c) {
                Class::Consonant => parts.consonants.push((i, c)),
                Class::Virama => {
                    let joins = chars
                        .peek()
                        .is_some_and(|&(_, n)| matches!(class(n), Class::Consonant | Class::Zwj));
                    if !joins {
                        parts.signs.push((i, c));
                    }
                }
                Class::VowelSign => parts.signs.extend(c.nfd().map(|p| (i, p))),
                Class::Modifier => parts.mods.push((i, c)),
                _ => parts.other.push((i, c)),
            }
        }
        parts
    }
}

struct Writer {
    glyphs: &'static Glyphs,
    out: String,
    unencodable: Vec<Unencodable>,
}

impl Writer {
    fn consonant(&mut self, parts: &Parts) {
        let arka = parts.consonants.len() > 1 && parts.consonants[0].1 == RA;
        let cluster = &parts.consonants[usize::from(arka)..];
        let Some(&(head_offset, head)) = cluster.first() else {
            return;
        };
        let sign_chars: Vec<char> = parts.signs.iter().map(|&(_, c)| c).collect();
        let (before, after): (Vec<_>, Vec<_>) = parts
            .signs
            .iter()
            .partition(|&&(_, c)| !AFTER_SUBSCRIPTS.contains(&c));
        let mut context = Context {
            consonant: head,
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
        for &(i, c) in &before[used..] {
            let key = self.glyphs.signs.get(&c).and_then(|s| choose(s, &context));
            self.write(key, i, c);
        }
        for &(i, c) in &cluster[1..] {
            let key = self.glyphs.ottus.get(&c).and_then(|o| choose(o, &context));
            self.write(key, i, c);
            context.after_ottu = true;
        }
        for &(i, c) in &after {
            let key = self.glyphs.signs.get(&c).and_then(|s| choose(s, &context));
            self.write(key, i, c);
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
        if c.is_ascii_digit() || (is_passthrough(c) && !g.key_starts.contains(&c)) {
            self.out.push(c);
            return;
        }
        let context = Context {
            consonant: '\0',
            signs: &[],
            after_ottu: false,
        };
        match class(c) {
            Class::Digit => self.out.push(to_ascii_digit(c)),
            Class::Vowel => {
                let key = g.letters.get(&c.nfd().collect::<String>());
                self.write(key.map(String::as_str), offset, c);
            }
            Class::VowelSign | Class::Virama => {
                for part in c.nfd() {
                    let key = g.signs.get(&part).and_then(|s| choose(s, &context));
                    self.write(key, offset, c);
                }
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
    use crate::legacy::{decode_windows_1252, nudi_to_unicode, Options};

    fn encode(text: &str) -> Vec<u8> {
        let e = unicode_to_nudi(text);
        assert!(e.is_complete(), "{e:?}");
        e.text
            .chars()
            .map(|c| {
                (0u8..=255)
                    .find(|&b| decode_windows_1252(&[b]).starts_with(c))
                    .expect("Windows-1252 character")
            })
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
        assert!(!unicode_to_nudi("„").is_complete());
    }

    #[test]
    fn zwnj_is_reported_and_its_virama_kept() {
        let e = unicode_to_nudi("ಡ್\u{200C}ಗೆ");
        assert_eq!(e.unencodable.len(), 1);
        let back = nudi_to_unicode(&e.text.replace('\u{200C}', ""), &Options::default());
        assert_eq!(back.text, "ಡ್ಗೆ");
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
}
