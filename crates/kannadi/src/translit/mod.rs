//! Transliteration between Kannada script and Latin.
//!
//! Supported schemes: ISO 15919, IAST (extended for Kannada) and Baraha phonetic.
//! All of them cover the Old Kannada letters ಱ and ೞ, nakaara pollu, the rare
//! vowels ೠ ಌ ೡ and their signs, candrabindu, jihvamuliya and upadhmaniya.
//!
//! Kannada to Latin and back is lossless for Kannada text: when two letters would
//! otherwise read as one, the scheme's separator is written between them
//! (`k:h` for ಕ್ಹ in ISO 15919, `bakkiMg~hAm` in Baraha). Characters a scheme
//! has no spelling for, such as ZWNJ in ISO 15919, pass through unchanged and
//! survive the round trip. Kannada digits become ASCII digits.
//!
//! Latin to Kannada converts every Latin letter it can read, so English words
//! mixed into Latin input are converted too.

mod schemes;

use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;
use std::sync::OnceLock;

use unicode_normalization::UnicodeNormalization;

use crate::chars::{class, is_kannada, Class, NUKTA, VIRAMA};
use crate::numerals::to_ascii_digit;
use schemes::Table;

/// A writing scheme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Scheme {
    /// Kannada script (Unicode).
    Kannada,
    /// ISO 15919 romanisation.
    Iso15919,
    /// IAST romanisation, extended for Kannada.
    Iast,
    /// Baraha phonetic input scheme.
    Baraha,
}

impl Scheme {
    /// All schemes.
    pub const ALL: [Scheme; 4] = [
        Scheme::Kannada,
        Scheme::Iso15919,
        Scheme::Iast,
        Scheme::Baraha,
    ];

    /// The short name accepted by [`FromStr`].
    pub fn name(self) -> &'static str {
        match self {
            Scheme::Kannada => "kannada",
            Scheme::Iso15919 => "iso15919",
            Scheme::Iast => "iast",
            Scheme::Baraha => "baraha",
        }
    }

    fn table(self) -> Option<&'static Compiled> {
        static ISO: OnceLock<Compiled> = OnceLock::new();
        static IAST: OnceLock<Compiled> = OnceLock::new();
        static BARAHA: OnceLock<Compiled> = OnceLock::new();
        match self {
            Scheme::Kannada => None,
            Scheme::Iso15919 => Some(ISO.get_or_init(|| Compiled::new(&schemes::ISO_15919))),
            Scheme::Iast => Some(IAST.get_or_init(|| Compiled::new(&schemes::IAST))),
            Scheme::Baraha => Some(BARAHA.get_or_init(|| Compiled::new(&schemes::BARAHA))),
        }
    }
}

impl fmt::Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.name())
    }
}

/// Error returned when a scheme name is not recognised.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownScheme(pub String);

impl fmt::Display for UnknownScheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "unknown scheme {:?}; expected kannada, iso15919, iast or baraha",
            self.0
        )
    }
}

impl std::error::Error for UnknownScheme {}

impl FromStr for Scheme {
    type Err = UnknownScheme;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().replace(['-', '_', ' '], "").as_str() {
            "kannada" | "kn" | "knda" => Ok(Scheme::Kannada),
            "iso15919" | "iso" => Ok(Scheme::Iso15919),
            "iast" => Ok(Scheme::Iast),
            "baraha" => Ok(Scheme::Baraha),
            _ => Err(UnknownScheme(s.to_string())),
        }
    }
}

/// Converts `text` from one scheme to another. Latin to Latin goes through Kannada.
///
/// ```
/// use kannadi::{transliterate, Scheme};
/// assert_eq!(transliterate("ಕನ್ನಡ", Scheme::Kannada, Scheme::Iso15919), "kannaḍa");
/// assert_eq!(transliterate("kannaDa", Scheme::Baraha, Scheme::Kannada), "ಕನ್ನಡ");
/// assert_eq!(transliterate("kannaDa", Scheme::Baraha, Scheme::Iast), "kannaḍa");
/// ```
pub fn transliterate(text: &str, from: Scheme, to: Scheme) -> String {
    match (from.table(), to.table()) {
        (None, None) => text.to_string(),
        (None, Some(t)) => t.romanise(text),
        (Some(f), None) => f.parse(text),
        (Some(f), Some(t)) => t.romanise(&f.parse(text)),
    }
}

/// Romanises Kannada text. Shorthand for `transliterate(text, Scheme::Kannada, scheme)`.
pub fn to_latin(text: &str, scheme: Scheme) -> String {
    transliterate(text, Scheme::Kannada, scheme)
}

/// Converts romanised text to Kannada. Shorthand for `transliterate(text, scheme, Scheme::Kannada)`.
pub fn from_latin(text: &str, scheme: Scheme) -> String {
    transliterate(text, scheme, Scheme::Kannada)
}

/// What a Latin token stands for.
#[derive(Clone, Copy, Debug)]
enum Tok {
    Vowel { letter: char, sign: Option<char> },
    Consonant { letter: char, nukta: bool },
    Mark(char),
    Joiner(char),
    Separator,
}

/// What the last piece of Latin output was, for deciding where a separator is needed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Piece {
    /// A consonant still waiting for its vowel.
    Consonant,
    /// A consonant followed by a virama.
    DeadConsonant,
    Vowel,
    Other,
}

struct Compiled {
    table: &'static Table,
    consonant: HashMap<char, &'static str>,
    nukta_consonant: HashMap<char, &'static str>,
    vowel: HashMap<char, &'static str>,
    sign: HashMap<char, &'static str>,
    mark: HashMap<char, &'static str>,
    tokens: HashMap<String, Tok>,
    longest_token: usize,
    inherent: &'static str,
}

impl Compiled {
    fn new(table: &'static Table) -> Self {
        let mut tokens: HashMap<String, Tok> = HashMap::new();
        let mut vowel = HashMap::new();
        let mut sign = HashMap::new();
        let mut inherent = "a";
        for &(latin, letter, s) in table.vowels {
            vowel.insert(letter, latin);
            match s {
                Some(s) => {
                    sign.insert(s, latin);
                }
                None => inherent = latin,
            }
            tokens.insert(latin.into(), Tok::Vowel { letter, sign: s });
        }
        let mut consonant = HashMap::new();
        for &(latin, letter) in table.consonants {
            consonant.insert(letter, latin);
            tokens.insert(
                latin.into(),
                Tok::Consonant {
                    letter,
                    nukta: false,
                },
            );
        }
        let mut nukta_consonant = HashMap::new();
        for &(latin, letter) in table.nukta_consonants {
            nukta_consonant.insert(letter, latin);
            tokens.insert(
                latin.into(),
                Tok::Consonant {
                    letter,
                    nukta: true,
                },
            );
        }
        let mut mark = HashMap::new();
        for &(latin, m) in table.marks {
            mark.insert(m, latin);
            tokens.insert(latin.into(), Tok::Mark(m));
        }
        for &(latin, j) in table.joiners {
            mark.insert(j, latin);
            tokens.insert(latin.into(), Tok::Joiner(j));
        }
        for &(alias, canonical) in table.aliases {
            let tok = tokens[canonical];
            tokens.insert(alias.into(), tok);
        }
        tokens.insert(table.separator.into(), Tok::Separator);
        let longest_token = tokens.keys().map(|k| k.chars().count()).max().unwrap_or(1);
        Compiled {
            table,
            consonant,
            nukta_consonant,
            vowel,
            sign,
            mark,
            tokens,
            longest_token,
            inherent,
        }
    }

    fn romanise(&self, text: &str) -> String {
        let mut out = Emitter {
            scheme: self,
            out: String::with_capacity(text.len()),
            last_start: 0,
            last: Piece::Other,
        };
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if out.last == Piece::Consonant
                && !self.sign.contains_key(&c)
                && c != VIRAMA
                && c != NUKTA
            {
                out.push(self.inherent, Piece::Vowel);
            }
            match class(c) {
                Class::Consonant => {
                    if chars.peek() == Some(&NUKTA) {
                        if let Some(latin) = self.nukta_consonant.get(&c) {
                            chars.next();
                            out.push(latin, Piece::Consonant);
                            continue;
                        }
                    }
                    out.push(self.consonant[&c], Piece::Consonant);
                }
                Class::Nukta if out.last == Piece::Consonant => {
                    out.push_nukta();
                }
                Class::Virama if out.last == Piece::Consonant => out.last = Piece::DeadConsonant,
                Class::VowelSign if out.last == Piece::Consonant && self.sign.contains_key(&c) => {
                    out.push(self.sign[&c], Piece::Vowel);
                }
                Class::Vowel => out.push(self.vowel[&c], Piece::Vowel),
                Class::Digit => out.push_char(to_ascii_digit(c)),
                _ => match self.mark.get(&c) {
                    Some(latin) => out.push(latin, Piece::Other),
                    None => out.push_char(c),
                },
            }
        }
        if out.last == Piece::Consonant {
            out.push(self.inherent, Piece::Vowel);
        }
        out.out
    }

    fn parse(&self, text: &str) -> String {
        let mut input = nfc_outside_kannada(text);
        if self.table.case_insensitive {
            input.iter_mut().for_each(|c| *c = lowercase_latin(*c));
        }
        let mut out = String::with_capacity(text.len() * 2);
        let mut pending = false;
        let mut i = 0;
        let mut buf = String::new();
        while i < input.len() {
            if pending {
                if input[i] == NUKTA {
                    out.push(NUKTA);
                    i += 1;
                    continue;
                }
                if let Some(suffix) = self.table.nukta_suffix {
                    if starts_with(&input[i..], suffix)
                        && self.longest_match(&input[i..], &mut buf).is_none()
                    {
                        out.push(NUKTA);
                        i += suffix.chars().count();
                        continue;
                    }
                }
            }
            let Some((tok, len)) = self.longest_match(&input[i..], &mut buf) else {
                if pending {
                    out.push(VIRAMA);
                    pending = false;
                }
                out.push(input[i]);
                i += 1;
                continue;
            };
            i += len;
            match tok {
                Tok::Vowel { letter, sign } => {
                    if pending {
                        out.extend(sign);
                        pending = false;
                    } else {
                        out.push(letter);
                    }
                }
                Tok::Consonant { letter, nukta } => {
                    if pending {
                        out.push(VIRAMA);
                    }
                    out.push(letter);
                    if nukta {
                        out.push(NUKTA);
                    }
                    pending = true;
                }
                Tok::Mark(m) | Tok::Joiner(m) => {
                    if pending {
                        out.push(VIRAMA);
                        pending = false;
                    }
                    out.push(m);
                }
                Tok::Separator => {
                    if pending {
                        out.push(VIRAMA);
                        pending = false;
                    }
                }
            }
        }
        if pending {
            out.push(VIRAMA);
        }
        out
    }

    /// Finds the longest token at the start of `input`, returning it and its length in chars.
    fn longest_match(&self, input: &[char], buf: &mut String) -> Option<(Tok, usize)> {
        let max = self.longest_token.min(input.len());
        for len in (1..=max).rev() {
            buf.clear();
            buf.extend(&input[..len]);
            if let Some(&tok) = self.tokens.get(buf.as_str()) {
                return Some((tok, len));
            }
        }
        None
    }

    /// True if the parser, reading `joined` from its start, would take a token longer than `first`.
    fn would_merge(&self, first: &str, joined: &str) -> bool {
        let first_len = first.chars().count();
        let chars: Vec<char> = joined.chars().collect();
        let mut buf = String::new();
        match self.longest_match(&chars, &mut buf) {
            Some((_, len)) => len > first_len,
            None => false,
        }
    }
}

/// Applies NFC to runs of non-Kannada text so Latin combining marks match the tables,
/// while Kannada text passing through keeps its exact code points.
fn nfc_outside_kannada(text: &str) -> Vec<char> {
    let mut out = Vec::with_capacity(text.len());
    let mut run_start = 0;
    for (i, c) in text.char_indices() {
        if is_kannada(c) {
            out.extend(text[run_start..i].nfc());
            out.push(c);
            run_start = i + c.len_utf8();
        }
    }
    out.extend(text[run_start..].nfc());
    out
}

fn starts_with(input: &[char], prefix: &str) -> bool {
    let mut it = input.iter();
    prefix.chars().all(|p| it.next() == Some(&p))
}

fn lowercase_latin(c: char) -> char {
    let latin = c.is_ascii_alphabetic()
        || ('\u{00C0}'..='\u{024F}').contains(&c)
        || ('\u{1E00}'..='\u{1EFF}').contains(&c);
    if !latin {
        return c;
    }
    let mut lower = c.to_lowercase();
    match (lower.next(), lower.next()) {
        (Some(l), None) => l,
        _ => c,
    }
}

struct Emitter<'a> {
    scheme: &'a Compiled,
    out: String,
    /// Byte offset in `out` where the last token-shaped piece starts.
    last_start: usize,
    last: Piece,
}

impl Emitter<'_> {
    fn push(&mut self, piece: &str, kind: Piece) {
        let table = self.scheme.table;
        let mut piece = piece;
        let prev = &self.out[self.last_start..];
        let dead_then_vowel = self.last == Piece::DeadConsonant && kind == Piece::Vowel;
        let merges = !prev.is_empty() && self.scheme.would_merge(prev, &format!("{prev}{piece}"));
        if dead_then_vowel || merges {
            let is_ha = kind == Piece::Consonant && self.scheme.consonant.get(&'ಹ') == Some(&piece);
            match table.ha_breaker {
                Some(breaker)
                    if is_ha && !self.scheme.would_merge(prev, &format!("{prev}{breaker}")) =>
                {
                    piece = breaker;
                }
                _ => self.out.push_str(table.separator),
            }
        }
        self.last_start = self.out.len();
        self.out.push_str(piece);
        self.last = kind;
    }

    fn push_char(&mut self, c: char) {
        if c == NUKTA && self.last == Piece::DeadConsonant {
            self.out.push_str(self.scheme.table.separator);
        }
        self.last_start = self.out.len();
        self.out.push(c);
        self.last = Piece::Other;
    }

    /// Adds a nukta to the consonant just written, keeping it open for a vowel.
    fn push_nukta(&mut self) {
        let suffix = self.scheme.table.nukta_suffix;
        let prev = &self.out[self.last_start..];
        match suffix {
            Some(s) if !self.scheme.would_merge(prev, &format!("{prev}{s}")) => {
                self.out.push_str(s)
            }
            _ => self.out.push(NUKTA),
        }
        self.last = Piece::Consonant;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iso(kn: &str) -> String {
        to_latin(kn, Scheme::Iso15919)
    }

    #[test]
    fn iso_basic_words() {
        assert_eq!(iso("ಕನ್ನಡ"), "kannaḍa");
        assert_eq!(iso("ಬೆಂಗಳೂರು"), "beṁgaḷūru");
        assert_eq!(iso("ಜ್ಞಾನ"), "jñāna");
        assert_eq!(iso("ಕ್ಷೇತ್ರ"), "kṣētra");
        assert_eq!(iso("ಸಂಸ್ಕೃತ"), "saṁskr̥ta");
    }

    #[test]
    fn iso_old_kannada() {
        assert_eq!(iso("ಅಱಿವು"), "aṟivu");
        assert_eq!(iso("ಬಾೞ್"), "bāḻ");
        assert_eq!(from_latin("bāḻ", Scheme::Iso15919), "ಬಾೞ್");
    }

    #[test]
    fn separators_where_letters_would_join() {
        assert_eq!(iso("ಕ್ಹ"), "k:ha");
        assert_eq!(iso("ಕಇ"), "ka:i");
        assert_eq!(iso("ಕಉ"), "ka:u");
        assert_eq!(iso("ಕ್ಅ"), "k:a");
        assert_eq!(from_latin("k:ha", Scheme::Iso15919), "ಕ್ಹ");
        assert_eq!(from_latin("ka:i", Scheme::Iso15919), "ಕಇ");
    }

    #[test]
    fn baraha_uses_tilde_h_to_break_conjuncts() {
        let b = to_latin("ಬಕ್ಕಿಂಗ್ಹಾಮ್", Scheme::Baraha);
        assert_eq!(b, "bakkiMg~hAm");
        assert_eq!(from_latin(&b, Scheme::Baraha), "ಬಕ್ಕಿಂಗ್ಹಾಮ್");
        assert_eq!(to_latin("ಸ್ಹ", Scheme::Baraha), "s~ha");
    }

    #[test]
    fn baraha_aliases_on_input() {
        assert_eq!(from_latin("kaa", Scheme::Baraha), "ಕಾ");
        assert_eq!(from_latin("Kanna", Scheme::Baraha), "ಖನ್ನ");
        assert_eq!(from_latin("ಕೆ", Scheme::Baraha), "ಕೆ");
        assert_eq!(from_latin("R", Scheme::Baraha), "ಱ್");
        assert_eq!(from_latin("rx", Scheme::Baraha), "ಱ್");
    }

    #[test]
    fn nukta_letters() {
        assert_eq!(iso("ಫ಼ೈಲ್"), "fail");
        assert_eq!(from_latin("fail", Scheme::Iso15919), "ಫ಼ೈಲ್");
        assert_eq!(to_latin("ಕ಼ಾ", Scheme::Baraha), "kxA");
        assert_eq!(from_latin("kxA", Scheme::Baraha), "ಕ಼ಾ");
        assert_eq!(to_latin("ರ಼", Scheme::Baraha), "r\u{0CBC}a");
        assert_eq!(from_latin("r\u{0CBC}a", Scheme::Baraha), "ರ಼");
    }

    #[test]
    fn iso_input_is_case_insensitive_and_normalised() {
        assert_eq!(from_latin("KANNAḌA", Scheme::Iso15919), "ಕನ್ನಡ");
        assert_eq!(
            from_latin("kanna\u{0064}\u{0323}a", Scheme::Iso15919),
            "ಕನ್ನಡ"
        );
    }

    #[test]
    fn digits_become_ascii_and_others_pass_through() {
        assert_eq!(iso("ಪುಟ ೧೨, ZWNJ ಡ್\u{200C}ಗೆ"), "puṭa 12, ZWNJ ḍ\u{200C}ge");
    }

    #[test]
    fn scheme_names_parse() {
        for s in Scheme::ALL {
            assert_eq!(s.name().parse::<Scheme>(), Ok(s));
        }
        assert_eq!("ISO-15919".parse::<Scheme>(), Ok(Scheme::Iso15919));
        assert!("hk".parse::<Scheme>().is_err());
    }
}
