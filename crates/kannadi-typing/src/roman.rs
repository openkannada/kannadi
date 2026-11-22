//! Latin input: cleaning it up, and the short letter groups ("chunks") that
//! the model pairs with Kannada units.

/// The longest chunk of Latin letters paired with 1 unit.
pub const MAX_CHUNK: usize = 3;
/// One more than the largest chunk code.
pub const CHUNK_CODES: usize = 27 * 27 * 27;

/// The code of a chunk of 0 to 3 lowercase ASCII letters; distinct chunks get distinct codes.
pub fn chunk_code(chunk: &[u8]) -> u16 {
    debug_assert!(chunk.len() <= MAX_CHUNK);
    chunk
        .iter()
        .fold(0u16, |v, &b| v * 27 + (b - b'a' + 1) as u16)
}

/// The letters of a chunk code.
#[cfg(test)]
pub(crate) fn chunk_text(mut code: u16) -> String {
    let mut out = Vec::new();
    while code > 0 {
        out.push(b'a' - 1 + (code % 27) as u8);
        code /= 27;
    }
    out.reverse();
    String::from_utf8(out).expect("ASCII letters")
}

/// The marks Baraha and ITRANS use besides letters, as in `gaa.ndhii` or `j~naana`.
const MARKS: &[u8] = b".~^";

/// The letters the model reads: lowercase a to z. x reads as ks and q as k,
/// the way most people use them for Kannada, and Baraha and ITRANS marks are
/// skipped. Returns `None` if `text` has any other character.
pub fn letters(text: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(text.len() + 2);
    for b in text.bytes() {
        match b.to_ascii_lowercase() {
            b'x' => out.extend_from_slice(b"ks"),
            b'q' => out.push(b'k'),
            c @ b'a'..=b'z' => out.push(c),
            c if MARKS.contains(&c) => {}
            _ => return None,
        }
    }
    Some(out)
}

/// How many letters were typed, before x becomes ks.
pub fn typed_letters(text: &str) -> usize {
    text.bytes().filter(u8::is_ascii_alphabetic).count()
}

/// Whether `text` is typed precisely in Baraha or ITRANS: it has capitals
/// inside the word (kannaDa), or a mark such as the dot in `gaa.ndhii`.
pub fn is_precise(text: &str) -> bool {
    has_inner_capitals(text) || text.bytes().any(|b| MARKS.contains(&b))
}

/// Whether `text` uses capitals as Baraha does (kannaDa), rather than only at
/// the start of a word (Kannada) or throughout (KANNADA).
pub fn has_inner_capitals(text: &str) -> bool {
    let rest = text.get(1..).unwrap_or("");
    rest.bytes().any(|b| b.is_ascii_uppercase()) && rest.bytes().any(|b| b.is_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_codes_round_trip_and_fit() {
        for c in ["", "a", "z", "kh", "zzz", "chh"] {
            assert_eq!(chunk_text(chunk_code(c.as_bytes())), c);
        }
        assert!((chunk_code(b"zzz") as usize) < CHUNK_CODES);
    }

    #[test]
    fn reads_letters() {
        assert_eq!(letters("Taxi").unwrap(), b"taksi");
        assert_eq!(letters("quran").unwrap(), b"kuran");
        assert_eq!(letters("a-b"), None);
        assert_eq!(letters("gaa.ndhii").unwrap(), b"gaandhii");
        assert_eq!(letters("j~naana").unwrap(), b"jnaana");
        assert_eq!(typed_letters("xa"), 2);
        assert_eq!(typed_letters("gaa.n"), 4);
    }

    #[test]
    fn marks_and_inner_capitals_are_precise() {
        assert!(is_precise("gaa.ndhii") && is_precise("j~naana") && is_precise("kannaDa"));
        assert!(!is_precise("Kannada") && !is_precise("kannada"));
    }

    #[test]
    fn tells_baraha_capitals_from_sentence_case() {
        assert!(has_inner_capitals("kannaDa"));
        assert!(!has_inner_capitals("Kannada"));
        assert!(!has_inner_capitals("KANNADA"));
        assert!(!has_inner_capitals("k"));
    }
}
