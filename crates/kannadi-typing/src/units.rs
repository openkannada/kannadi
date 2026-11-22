//! Kannada words as sequences of sound units, the native side of the model.
//!
//! A unit is an independent vowel, a consonant that carries a vowel (written
//! with a vowel sign or the inherent a), a consonant with a virama, a vowel
//! sign, the inherent a itself, or a sign such as the anusvara. The inherent a
//! is a unit of its own because typists spell it ("kannada") or drop it
//! ("kannda"), and the aligner must see it either way.

use unicode_normalization::UnicodeNormalization;

/// A unit id. Ids are below 128, so a word's units are also a byte string.
pub type Unit = u8;

/// What kind of sound a unit is, which decides what may follow it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// An independent vowel letter, such as ಅ.
    Vowel,
    /// A consonant followed by a vowel sign or the inherent a.
    Full,
    /// A consonant followed by a virama.
    Dead,
    /// A dependent vowel sign, such as ಾ.
    Sign,
    /// The inherent a after a consonant, which is not written.
    Inherent,
    /// The anusvara, visarga or candrabindu.
    Mark,
}

/// The inherent a.
pub const INHERENT: Unit = 1;
const VOWELS: [char; 16] = [
    'ಅ', 'ಆ', 'ಇ', 'ಈ', 'ಉ', 'ಊ', 'ಋ', 'ೠ', 'ಌ', 'ೡ', 'ಎ', 'ಏ', 'ಐ', 'ಒ', 'ಓ', 'ಔ',
];
const SIGNS: [char; 15] = [
    'ಾ', 'ಿ', 'ೀ', 'ು', 'ೂ', 'ೃ', 'ೄ', 'ೢ', 'ೣ', 'ೆ', 'ೇ', 'ೈ', 'ೊ', 'ೋ', 'ೌ',
];
const MARKS: [char; 3] = ['ಂ', 'ಃ', 'ಁ'];
/// Consonants; the last 2 are ಜ಼ and ಫ಼, written with a nukta.
const CONSONANTS: [char; 38] = [
    'ಕ', 'ಖ', 'ಗ', 'ಘ', 'ಙ', 'ಚ', 'ಛ', 'ಜ', 'ಝ', 'ಞ', 'ಟ', 'ಠ', 'ಡ', 'ಢ', 'ಣ', 'ತ', 'ಥ', 'ದ', 'ಧ',
    'ನ', 'ಪ', 'ಫ', 'ಬ', 'ಭ', 'ಮ', 'ಯ', 'ರ', 'ಱ', 'ಲ', 'ಳ', 'ವ', 'ಶ', 'ಷ', 'ಸ', 'ಹ', 'ೞ', 'ಜ', 'ಫ',
];
const NUKTA_FROM: usize = 36;
const VOWEL0: Unit = 2;
const SIGN0: Unit = VOWEL0 + VOWELS.len() as Unit;
const MARK0: Unit = SIGN0 + SIGNS.len() as Unit;
const FULL0: Unit = MARK0 + MARKS.len() as Unit;
const DEAD0: Unit = FULL0 + CONSONANTS.len() as Unit;
/// One more than the largest unit id.
pub const COUNT: usize = DEAD0 as usize + CONSONANTS.len();
const _: () = assert!(COUNT <= 128, "unit ids must fit in 7 bits");

const VIRAMA: char = '\u{0CCD}';
const NUKTA: char = '\u{0CBC}';
const ZWNJ: char = '\u{200C}';
const ZWJ: char = '\u{200D}';

/// The kind of `unit`. Panics on 0 or an id of `COUNT` or more.
pub fn kind(unit: Unit) -> Kind {
    match unit {
        INHERENT => Kind::Inherent,
        u if u < VOWEL0 => panic!("unit 0 is not a unit"),
        u if u < SIGN0 => Kind::Vowel,
        u if u < MARK0 => Kind::Sign,
        u if u < FULL0 => Kind::Mark,
        u if u < DEAD0 => Kind::Full,
        u if (u as usize) < COUNT => Kind::Dead,
        u => panic!("unit {u} is out of range"),
    }
}

/// Whether `next` may follow `prev` (`None` at the start of a word).
pub fn may_follow(prev: Option<Unit>, next: Unit) -> bool {
    let next = kind(next);
    match prev.map(kind) {
        None => matches!(next, Kind::Vowel | Kind::Full | Kind::Dead),
        Some(Kind::Full) => matches!(next, Kind::Sign | Kind::Inherent),
        Some(Kind::Dead) => matches!(next, Kind::Full | Kind::Dead),
        Some(Kind::Mark) => matches!(next, Kind::Vowel | Kind::Full | Kind::Dead),
        Some(Kind::Vowel | Kind::Sign | Kind::Inherent) => {
            !matches!(next, Kind::Sign | Kind::Inherent)
        }
    }
}

/// Whether a word may end after `last`.
pub fn may_end(last: Unit) -> bool {
    kind(last) != Kind::Full
}

fn index_of(list: &[char], c: char) -> Option<usize> {
    list.iter().position(|&x| x == c)
}

/// Splits a Kannada word into units. Joiners are dropped. Returns `None` for
/// an empty word, or one with a character outside the units or out of order.
pub fn units(word: &str) -> Option<Vec<Unit>> {
    let chars: Vec<char> = word.nfc().filter(|&c| c != ZWNJ && c != ZWJ).collect();
    let mut out = Vec::with_capacity(chars.len() + 4);
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        i += 1;
        if let Some(v) = index_of(&VOWELS, c) {
            out.push(VOWEL0 + v as Unit);
        } else if let Some(m) = index_of(&MARKS, c) {
            out.push(MARK0 + m as Unit);
        } else {
            let mut k = index_of(&CONSONANTS[..NUKTA_FROM], c)?;
            if chars.get(i) == Some(&NUKTA) {
                k = match c {
                    'ಜ' => NUKTA_FROM,
                    'ಫ' => NUKTA_FROM + 1,
                    _ => return None,
                };
                i += 1;
            }
            match chars.get(i).and_then(|&s| {
                if s == VIRAMA {
                    Some(None)
                } else {
                    index_of(&SIGNS, s).map(Some)
                }
            }) {
                Some(None) => {
                    out.push(DEAD0 + k as Unit);
                    i += 1;
                }
                Some(Some(sign)) => {
                    out.push(FULL0 + k as Unit);
                    out.push(SIGN0 + sign as Unit);
                    i += 1;
                }
                None => {
                    out.push(FULL0 + k as Unit);
                    out.push(INHERENT);
                }
            }
        }
    }
    let valid = !out.is_empty()
        && out
            .iter()
            .enumerate()
            .all(|(j, &u)| may_follow(j.checked_sub(1).map(|p| out[p]), u))
        && may_end(*out.last().unwrap());
    valid.then_some(out)
}

/// Writes units back as Kannada text.
pub fn text(units: &[Unit]) -> String {
    let mut s = String::with_capacity(units.len() * 3);
    for &u in units {
        push(&mut s, u);
    }
    s
}

/// Appends the text of 1 unit.
pub fn push(s: &mut String, u: Unit) {
    match kind(u) {
        Kind::Inherent => {}
        Kind::Vowel => s.push(VOWELS[(u - VOWEL0) as usize]),
        Kind::Sign => s.push(SIGNS[(u - SIGN0) as usize]),
        Kind::Mark => s.push(MARKS[(u - MARK0) as usize]),
        Kind::Full | Kind::Dead => {
            let k = (if kind(u) == Kind::Full {
                u - FULL0
            } else {
                u - DEAD0
            }) as usize;
            s.push(CONSONANTS[k]);
            if k >= NUKTA_FROM {
                s.push(NUKTA);
            }
            if kind(u) == Kind::Dead {
                s.push(VIRAMA);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_words() {
        for w in ["ಕನ್ನಡ", "ಬೆಂಗಳೂರು", "ಸರ್ಕಾರ", "ಜ಼ೀರೋ", "ಅಃ", "ಬಸ್", "ಆಉ", "ಕೊಡು"]
        {
            let u = units(w).unwrap_or_else(|| panic!("{w}"));
            assert_eq!(text(&u), w);
        }
    }

    #[test]
    fn inherent_a_is_a_unit() {
        let u = units("ಕನ್ನಡ").unwrap();
        let kinds: Vec<Kind> = u.iter().map(|&x| kind(x)).collect();
        use Kind::*;
        assert_eq!(
            kinds,
            [Full, Inherent, Dead, Full, Inherent, Full, Inherent]
        );
    }

    #[test]
    fn drops_joiners_and_rejects_other_text() {
        assert_eq!(text(&units("ಕನ್\u{200C}ನಡ").unwrap()), "ಕನ್ನಡ");
        for w in ["", "abc", "ಕ1", "ಂಕ", "ಾ", "ಕ಼"] {
            assert_eq!(units(w), None, "{w}");
        }
    }
}
