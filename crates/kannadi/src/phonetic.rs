//! Sound-alike keys for Kannada words, for fuzzy search and for suggesting
//! Kannada words from loosely typed Latin letters.
//!
//! Writers and searchers confuse letters that sound alike: aspirated and plain
//! stops (ದ, ಧ), short and long vowels (ಿ, ೀ), retroflex and dental (ಟ, ತ),
//! the 3 sibilants (ಶ, ಷ, ಸ), and nasals written as ಂ or as a letter. A key
//! maps a word to a string of sound classes, so that spellings that sound alike
//! share a key. There are 3 keys, from strict to loose:
//!
//! - [`Level::Narrow`] folds aspiration, vowel length, doubled consonants, and a
//!   nasal before a stop written as ಂ or as a letter. ಸಂಬಂಧ and ಸಮ್ಬಂದ match.
//! - [`Level::Medium`] also folds retroflex with dental, ಳ with ಲ, the 3
//!   sibilants, ಜ with ಝ and ಫ with ಪ. ನಿರ್ದಿಷ್ಟ and ನಿರ್ಧಿಸ್ಟ match.
//! - [`Level::Broad`] keeps only the consonants and a leading vowel. ನಿರ್ದಿಷ್ಟ
//!   and ನಿರ್ದೇಶ, which differ in more than spelling, both start `nrd`.
//!
//! Folding vowel length and doubled consonants also merges some different words:
//! ಹಾಲು and ಹಲ್ಲು share every key, as do ಕಾಲು and ಕಲ್ಲು. A search should rank an
//! exact match above a key match.
//!
//! Latin input ([`latin_key`]) gets the medium and broad keys: "neeru" matches
//! ನೀರು at medium level, and "nirdishta" matches ನಿರ್ದಿಷ್ಟ at broad level. The
//! keys are plain ASCII.

/// How much a key folds. See the module documentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Level {
    /// Folds aspiration, vowel length, doubled consonants and nasals before stops.
    Narrow,
    /// Also folds retroflex with dental, ಳ with ಲ, the sibilants, ಜ with ಝ and ಫ with ಪ.
    Medium,
    /// Keeps only the consonants and a leading vowel.
    Broad,
}

impl std::str::FromStr for Level {
    type Err = UnknownLevel;

    /// Parses "narrow", "medium" or "broad".
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "narrow" => Ok(Self::Narrow),
            "medium" => Ok(Self::Medium),
            "broad" => Ok(Self::Broad),
            _ => Err(UnknownLevel(s.to_owned())),
        }
    }
}

/// A key level name that is not "narrow", "medium" or "broad".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownLevel(pub String);

impl std::fmt::Display for UnknownLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown key level {:?}: use narrow, medium or broad",
            self.0
        )
    }
}

impl std::error::Error for UnknownLevel {}

/// 1 sound in a word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Sound {
    Consonant(Place),
    Vowel(char),
    /// ಂ: a nasal at the place of the stop after it, and "m" anywhere else.
    Anusvara,
}

/// A consonant's code at narrow and at medium level. Upper case marks the retroflex
/// consonants and the 2 sibilants that only the narrow key keeps apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Place {
    narrow: char,
    medium: char,
}

impl Place {
    const fn new(narrow: char, medium: char) -> Self {
        Self { narrow, medium }
    }

    fn is_stop(self) -> bool {
        "kgcjTDtdpb".contains(self.narrow)
    }

    fn is_nasal(self) -> bool {
        "nNm".contains(self.narrow)
    }

    fn is_labial(self) -> bool {
        "pbm".contains(self.narrow)
    }
}

const R: Place = Place::new('r', 'r');
const G: Place = Place::new('g', 'g');
const H: Place = Place::new('h', 'h');

fn consonant(c: char) -> Option<Place> {
    let (narrow, medium) = match c {
        'ಕ' | 'ಖ' => ('k', 'k'),
        'ಗ' | 'ಘ' => ('g', 'g'),
        'ಙ' | 'ಞ' | 'ನ' => ('n', 'n'),
        'ಚ' | 'ಛ' => ('c', 'c'),
        'ಜ' | 'ಝ' => ('j', 'j'),
        'ಟ' | 'ಠ' => ('T', 't'),
        'ಡ' | 'ಢ' => ('D', 'd'),
        'ಣ' => ('N', 'n'),
        'ತ' | 'ಥ' => ('t', 't'),
        'ದ' | 'ಧ' => ('d', 'd'),
        'ಪ' | 'ಫ' => ('p', 'p'),
        'ಬ' | 'ಭ' => ('b', 'b'),
        'ಮ' => ('m', 'm'),
        'ಯ' => ('y', 'y'),
        'ರ' | 'ಱ' => ('r', 'r'),
        'ಲ' => ('l', 'l'),
        'ಳ' | 'ೞ' => ('L', 'l'),
        'ವ' => ('v', 'v'),
        'ಶ' => ('S', 's'),
        'ಷ' => ('Z', 's'),
        'ಸ' => ('s', 's'),
        'ಹ' => ('h', 'h'),
        _ => return None,
    };
    Some(Place::new(narrow, medium))
}

/// The vowel class of an independent vowel letter: length is folded.
fn vowel_letter(c: char) -> Option<char> {
    Some(match c {
        'ಅ' | 'ಆ' => 'a',
        'ಇ' | 'ಈ' => 'i',
        'ಉ' | 'ಊ' => 'u',
        'ಎ' | 'ಏ' | 'ಐ' => 'e',
        'ಒ' | 'ಓ' | 'ಔ' => 'o',
        _ => return None,
    })
}

/// The vowel class of a dependent vowel sign.
fn vowel_sign(c: char) -> Option<char> {
    Some(match c {
        'ಾ' => 'a',
        'ಿ' | 'ೀ' => 'i',
        'ು' | 'ೂ' => 'u',
        'ೆ' | 'ೇ' | 'ೈ' => 'e',
        'ೊ' | 'ೋ' | 'ೌ' => 'o',
        _ => return None,
    })
}

const NUKTA: char = '\u{0CBC}';
const VIRAMA: char = '\u{0CCD}';
const ANUSVARA: char = '\u{0C82}';
const VISARGA: char = '\u{0C83}';

/// Vowel signs that decomposed text writes as 2 or 3 code points, and their single form.
const DECOMPOSED: &[(&str, &str)] = &[
    ("\u{0CC6}\u{0CC2}\u{0CD5}", "\u{0CCB}"),
    ("\u{0CCA}\u{0CD5}", "\u{0CCB}"),
    ("\u{0CC6}\u{0CC2}", "\u{0CCA}"),
    ("\u{0CC6}\u{0CD5}", "\u{0CC7}"),
    ("\u{0CC6}\u{0CD6}", "\u{0CC8}"),
    ("\u{0CBF}\u{0CD5}", "\u{0CC0}"),
];

/// `word` with decomposed vowel signs composed and zero-width joiners removed.
fn compose(word: &str) -> String {
    let mut text: String = word
        .chars()
        .filter(|&c| c != '\u{200C}' && c != '\u{200D}')
        .collect();
    for (from, to) in DECOMPOSED {
        text = text.replace(from, to);
    }
    text
}

/// The sounds of a word in Kannada script. Other characters are skipped.
fn sounds(word: &str) -> Vec<Sound> {
    let chars: Vec<char> = compose(word).chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if let Some(place) = consonant(c) {
            // ಜ್ಞ is said "gn" in Kannada: ಜ್ಞಾನ, gnana.
            let gn =
                c == 'ಜ' && chars.get(i + 1) == Some(&VIRAMA) && chars.get(i + 2) == Some(&'ಞ');
            out.push(Sound::Consonant(if gn { G } else { place }));
            let mut next = i + 1;
            if chars.get(next) == Some(&NUKTA) {
                next += 1;
            }
            let sign = chars.get(next).copied();
            if sign == Some(VIRAMA) {
                i = next;
            } else if matches!(sign, Some('ೃ' | 'ೄ')) {
                out.extend([Sound::Consonant(R), Sound::Vowel('u')]);
                i = next;
            } else if let Some(v) = sign.and_then(vowel_sign) {
                out.push(Sound::Vowel(v));
                i = next;
            } else {
                out.push(Sound::Vowel('a'));
                i = next - 1;
            }
        } else if let Some(v) = vowel_letter(c) {
            out.push(Sound::Vowel(v));
        } else if matches!(c, 'ಋ' | 'ೠ') {
            out.extend([Sound::Consonant(R), Sound::Vowel('u')]);
        } else if c == ANUSVARA {
            out.push(Sound::Anusvara);
        } else if c == VISARGA {
            out.push(Sound::Consonant(H));
        }
        i += 1;
    }
    out
}

/// The stop after a nasal, if there is one: a nasal before a stop takes its place.
fn following_stop(next: Option<&Sound>) -> Option<Place> {
    match next {
        Some(Sound::Consonant(p)) if p.is_stop() => Some(*p),
        _ => None,
    }
}

fn homorganic(stop: Place) -> char {
    if stop.is_labial() {
        'm'
    } else {
        'n'
    }
}

/// True when `next` is a sibilant or ಹ, before which writers spell a nasal as ಂ, ನ್ or ಮ್
/// and type it as "n" or "m": ಸಂಸ್ಕೃತ, "sanskrit", "samskrutha".
fn before_sibilant(next: Option<&Sound>) -> bool {
    matches!(next, Some(Sound::Consonant(p)) if "sh".contains(p.medium))
}

fn key_of_sounds(sounds: &[Sound], level: Level) -> String {
    let mut out = String::new();
    for (i, sound) in sounds.iter().enumerate() {
        let next = sounds.get(i + 1);
        let nasal = matches!(sound, Sound::Anusvara)
            || matches!(sound, Sound::Consonant(p) if p.is_nasal());
        let code = match *sound {
            _ if nasal && level != Level::Narrow && before_sibilant(next) => 'n',
            Sound::Consonant(p) => match following_stop(next) {
                Some(stop) if p.is_nasal() => homorganic(stop),
                _ if level == Level::Narrow => p.narrow,
                _ => p.medium,
            },
            Sound::Anusvara => following_stop(next).map_or('m', homorganic),
            Sound::Vowel(v) => {
                if level == Level::Broad && i > 0 {
                    continue;
                }
                v
            }
        };
        if out.ends_with(code) && !matches!(sound, Sound::Vowel(_)) {
            continue;
        }
        out.push(code);
    }
    out
}

/// The sound-alike key of a word written in Kannada script.
pub fn key(word: &str, level: Level) -> String {
    key_of_sounds(&sounds(word), level)
}

/// All 3 keys, from narrow to broad.
pub fn keys(word: &str) -> [String; 3] {
    let s = sounds(word);
    [Level::Narrow, Level::Medium, Level::Broad].map(|level| key_of_sounds(&s, level))
}

/// Latin letter groups and the medium-level sounds they spell; the longest group that
/// matches wins. Covers informal romanization (sh, th, aa, ee, chh, gn), Baraha and
/// ISO 15919. "N" stands for the anusvara.
const LATIN: &[(&str, &str)] = &[
    ("ksh", "ks"),
    ("chh", "c"),
    ("aa", "a"),
    ("ee", "i"),
    ("ii", "i"),
    ("oo", "u"),
    ("uu", "u"),
    ("ai", "e"),
    ("au", "o"),
    ("ou", "o"),
    ("kh", "k"),
    ("gh", "g"),
    ("ch", "c"),
    ("jh", "j"),
    ("th", "t"),
    ("dh", "d"),
    ("ph", "p"),
    ("bh", "b"),
    ("sh", "s"),
    ("zh", "l"),
    ("jn", "gn"),
    ("jñ", "gn"),
    ("j~j", "gn"),
    ("~g", "n"),
    ("ḥ", "h"),
    ("ā", "a"),
    ("ī", "i"),
    ("ū", "u"),
    ("ē", "e"),
    ("ō", "o"),
    ("ṭ", "t"),
    ("ṭh", "t"),
    ("ḍ", "d"),
    ("ḍh", "d"),
    ("ṇ", "n"),
    ("ḷ", "l"),
    ("ḻ", "l"),
    ("ṟ", "r"),
    ("ś", "s"),
    ("ṣ", "s"),
    ("ṁ", "N"),
    ("ṃ", "N"),
    ("ṅ", "n"),
    ("ñ", "n"),
    ("r̥", "ru"),
    ("ṛ", "ru"),
    ("a", "a"),
    ("i", "i"),
    ("u", "u"),
    ("e", "e"),
    ("o", "o"),
    ("k", "k"),
    ("q", "k"),
    ("c", "c"),
    ("g", "g"),
    ("j", "j"),
    ("z", "j"),
    ("t", "t"),
    ("d", "d"),
    ("n", "n"),
    ("p", "p"),
    ("f", "p"),
    ("b", "b"),
    ("m", "m"),
    ("y", "y"),
    ("r", "r"),
    ("l", "l"),
    ("v", "v"),
    ("w", "v"),
    ("s", "s"),
    ("x", "ks"),
    ("h", "h"),
];

/// The key of a word typed in Latin letters, comparable with the same level's key of a
/// Kannada word. Latin does not mark retroflex consonants, so the narrow key is the medium one.
pub fn latin_key(text: &str, level: Level) -> String {
    let lower = text.to_lowercase();
    let mut rest = lower.as_str();
    let mut sounds = Vec::new();
    while !rest.is_empty() {
        let longest = LATIN
            .iter()
            .filter(|(from, _)| rest.starts_with(from))
            .max_by_key(|(from, _)| from.len());
        let Some((from, to)) = longest else {
            let skip = rest.chars().next().map_or(1, char::len_utf8);
            rest = &rest[skip..];
            continue;
        };
        for c in to.chars() {
            sounds.push(match c {
                'N' => Sound::Anusvara,
                _ if "aiueo".contains(c) => Sound::Vowel(c),
                _ => Sound::Consonant(Place::new(c, c)),
            });
        }
        rest = &rest[from.len()..];
    }
    let level = if level == Level::Broad {
        Level::Broad
    } else {
        Level::Medium
    };
    key_of_sounds(&sounds, level)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn same(a: &str, b: &str, level: Level) -> bool {
        key(a, level) == key(b, level)
    }

    #[test]
    fn narrow_folds_aspiration_length_doubling_and_nasals_before_stops() {
        assert!(same("ಸಂಬಂಧ", "ಸಮ್ಬಂದ", Level::Narrow));
        assert!(same("ದೀಪ", "ದಿಪ", Level::Narrow));
        assert!(same("ಅಮ್ಮ", "ಅಮ", Level::Narrow));
        assert!(same("ಭಾರತ", "ಬಾರತ", Level::Narrow));
        assert!(same("ಹಣ್ಣು", "ಹಣು", Level::Narrow));
        assert!(!same("ಕಟ", "ಕತ", Level::Narrow));
        assert!(!same("ಮರ", "ಮನೆ", Level::Narrow));
    }

    #[test]
    fn a_nasal_keeps_its_place_before_a_non_stop() {
        assert!(!same("ಜನ್ಮ", "ಜಮ", Level::Narrow));
        assert!(!same("ಕಾಮ್ಯ", "ಕನ್ಯಾ", Level::Narrow));
        assert_eq!(key("ಉನ್ಮಾದ", Level::Narrow), "unmada");
        assert_eq!(key("ಸಂಸಾರ", Level::Narrow), "samsara");
    }

    #[test]
    fn medium_folds_retroflex_and_sibilants() {
        assert!(same("ನಿರ್ದಿಷ್ಟ", "ನಿರ್ಧಿಸ್ಟ", Level::Medium));
        assert!(same("ಬಾಳೆ", "ಬಾಲೆ", Level::Medium));
        assert!(same("ವಿಷ", "ವಿಶ", Level::Medium));
        assert!(!same("ವಿಷ", "ವಿಷು", Level::Medium));
    }

    #[test]
    fn broad_keeps_consonants_and_a_leading_vowel() {
        assert_eq!(key("ಅಮ್ಮ", Level::Broad), "am");
        assert_eq!(key("ಮನೆ", Level::Broad), "mn");
        assert!(same("ಕೃಷ್ಣ", "ಕ್ರಿಶ್ನ", Level::Broad));
    }

    #[test]
    fn levels_parse_by_name() {
        assert_eq!("medium".parse(), Ok(Level::Medium));
        assert_eq!("wide".parse::<Level>(), Err(UnknownLevel("wide".into())));
    }

    #[test]
    fn keys_are_ordered_narrow_to_broad() {
        assert_eq!(keys("ದೀಪ"), ["dipa", "dipa", "dp"]);
    }

    #[test]
    fn nukta_decomposed_signs_and_joiners_do_not_change_the_key() {
        assert_eq!(keys("ಫ\u{0CBC}ಿಲ್ಮ್"), keys("ಫಿಲ್ಮ್"));
        assert_eq!(key("ಜ\u{0CBC}ೀರೋ", Level::Narrow), "jiro");
        assert_eq!(keys("ಕ\u{0CC6}\u{0CC2}ಡು"), keys("ಕೊಡು"));
        assert_eq!(keys("ಕ\u{0CC6}\u{0CC2}\u{0CD5}ಟೆ"), keys("ಕೋಟೆ"));
        assert_eq!(keys("ಬ್ಯಾಂಕ್\u{200C}ನಲ್ಲಿ"), keys("ಬ್ಯಾಂಕ್ನಲ್ಲಿ"));
    }

    #[test]
    fn an_independent_vowel_after_a_consonant_is_its_own_sound() {
        assert_eq!(key("ಕಅ", Level::Narrow), "kaa");
    }

    #[test]
    fn jna_is_said_gn() {
        assert_eq!(key("ಜ್ಞಾನ", Level::Medium), "gnana");
    }

    #[test]
    fn latin_matches_kannada_at_broad_level() {
        for (latin, kannada) in [
            ("nirdishta", "ನಿರ್ದಿಷ್ಟ"),
            ("bharata", "ಭಾರತ"),
            ("bengaluru", "ಬೆಂಗಳೂರು"),
            ("sambandha", "ಸಂಬಂಧ"),
            ("kṛṣṇa", "ಕೃಷ್ಣ"),
            ("cakra", "ಚಕ್ರ"),
            ("chakra", "ಚಕ್ರ"),
            ("chhatra", "ಛತ್ರ"),
            ("duHkha", "ದುಃಖ"),
            ("svataH", "ಸ್ವತಃ"),
            ("hiM", "ಹಿಂ"),
            ("hiṁ", "ಹಿಂ"),
            ("saṁbandha", "ಸಂಬಂಧ"),
            ("praj~je", "ಪ್ರಜ್ಞೆ"),
            ("gnana", "ಜ್ಞಾನ"),
            ("jnana", "ಜ್ಞಾನ"),
            ("gūḍha", "ಗೂಢ"),
            ("pāṭha", "ಪಾಠ"),
            ("kaṟi", "ಕಱಿ"),
            ("kōḻi", "ಕೋೞಿ"),
        ] {
            assert_eq!(
                latin_key(latin, Level::Broad),
                key(kannada, Level::Broad),
                "{latin} {kannada}"
            );
        }
    }

    #[test]
    fn latin_matches_kannada_at_medium_level() {
        for (latin, kannada) in [
            ("neeru", "ನೀರು"),
            ("mane", "ಮನೆ"),
            ("shaale", "ಶಾಲೆ"),
            ("amma", "ಅಮ್ಮ"),
        ] {
            assert_eq!(
                latin_key(latin, Level::Medium),
                key(kannada, Level::Medium),
                "{latin} {kannada}"
            );
        }
        assert_eq!(
            latin_key("neeru", Level::Narrow),
            latin_key("neeru", Level::Medium)
        );
    }

    #[test]
    fn a_nasal_before_a_sibilant_or_h_matches_n_and_m_typing() {
        for (latin, kannada) in [
            ("sanskrit", "ಸಂಸ್ಕೃತ"),
            ("samskrutha", "ಸಂಸ್ಕೃತ"),
            ("sinha", "ಸಿಂಹ"),
            ("simha", "ಸಿಂಹ"),
            ("hansa", "ಹಂಸ"),
            ("vamsha", "ವಂಶ"),
        ] {
            assert_eq!(
                latin_key(latin, Level::Broad),
                key(kannada, Level::Broad),
                "{latin} {kannada}"
            );
        }
        assert!(same("ಸಂಸಾರ", "ಸನ್ಸಾರ", Level::Medium));
        assert!(!same("ಸಂಸಾರ", "ಸನ್ಸಾರ", Level::Narrow));
        assert!(!same("ಜನ್ಮ", "ಜಮ", Level::Medium));
    }

    #[test]
    fn ignores_other_characters() {
        assert_eq!(key("ಮನೆ123", Level::Broad), key("ಮನೆ", Level::Broad));
        assert_eq!(
            latin_key("Mane!", Level::Broad),
            latin_key("mane", Level::Broad)
        );
    }
}
