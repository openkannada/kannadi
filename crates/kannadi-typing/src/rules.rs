//! Strict readings of the input by rule: Baraha, and ITRANS spellings that
//! Baraha lacks. These catch precise typing such as `kannaDa` or `kRRiShNa`,
//! where the capitals say exactly which letter is meant.

use crate::roman::has_inner_capitals;
use crate::units::{units, Unit};
use kannadi::translit::{from_latin, Scheme};

/// ITRANS spellings and the Baraha spelling of the same letter, longest first
/// where 1 starts another.
const ITRANS: &[(&str, &str)] = &[
    ("RRi", "~ru"),
    ("R^i", "~ru"),
    ("RRI", "~rU"),
    ("R^I", "~rU"),
    ("LLi", "~lu"),
    ("L^i", "~lu"),
    ("chh", "Ch"),
    ("shh", "Sh"),
    ("GY", "j~j"),
    ("j~n", "j~j"),
    ("~N", "~g"),
    ("~n", "~j"),
    ("ii", "I"),
    ("uu", "U"),
    (".n", "M"),
    (".m", "M"),
    (".h", "^^"),
];

fn itrans_to_baraha(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    'outer: while !rest.is_empty() {
        for (from, to) in ITRANS {
            if let Some(tail) = rest.strip_prefix(from) {
                out.push_str(to);
                rest = tail;
                continue 'outer;
            }
        }
        let c = rest.chars().next().expect("not empty");
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    out
}

/// The rule readings of `input` as units, without repeats. Capitals count only
/// when they appear inside the word; x reads as ks and q as k.
pub fn readings(input: &str) -> Vec<Vec<Unit>> {
    let text = if has_inner_capitals(input) {
        input.to_string()
    } else {
        input.to_ascii_lowercase()
    };
    let text = text.replace('x', "ks").replace('q', "k");
    let mut out: Vec<Vec<Unit>> = Vec::new();
    for t in [text.clone(), itrans_to_baraha(&text)] {
        if let Some(u) = units(&from_latin(&t, Scheme::Baraha)) {
            if !out.contains(&u) {
                out.push(u);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::text;

    fn read(input: &str) -> Vec<String> {
        readings(input).iter().map(|u| text(u)).collect()
    }

    #[test]
    fn reads_baraha_and_itrans() {
        assert_eq!(read("kannaDa"), ["ಕನ್ನಡ"]);
        assert_eq!(read("Kannada"), ["ಕನ್ನದ"]);
        assert!(read("kRRiShNa").contains(&"ಕೃಷ್ಣ".to_string()));
        assert!(read("shrii").contains(&"ಶ್ರೀ".to_string()));
        assert!(read("gaa.ndhii").contains(&"ಗಾಂಧೀ".to_string()));
        assert_eq!(read("taxi"), ["ತಕ್ಸಿ"]);
    }

    #[test]
    fn gives_nothing_for_text_that_is_not_a_word() {
        assert!(readings("").is_empty());
        assert!(readings("123").is_empty());
    }
}
