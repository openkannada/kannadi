//! Spelling variants a searcher meets in real Kannada text, applied at random to clean text.

use kannadi::chars::{is_consonant, ANUSVARA, NUKTA, VIRAMA, ZWJ, ZWNJ};

/// SplitMix64: a fixed, dependency-free random sequence.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Rng(seed)
    }
    pub fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    /// True with probability `percent` in 100.
    pub fn chance(&mut self, percent: u64) -> bool {
        self.next() % 100 < percent
    }
}

fn group_nasal(stop: char) -> Option<char> {
    Some(match stop {
        'ಕ'..='ಘ' => 'ಙ',
        'ಚ'..='ಝ' => 'ಞ',
        'ಟ'..='ಢ' => 'ಣ',
        'ತ'..='ಧ' => 'ನ',
        'ಪ'..='ಭ' => 'ಮ',
        _ => return None,
    })
}

/// `text` with variant spellings:
/// - ZWNJ after every word-final virama, as Kannada Wikipedia writes ಆಫ್‌ (always);
/// - ZWNJ or ZWJ between a virama and the next consonant (5% each);
/// - ೦ (Kannada zero) for ಂ, as legacy converters and OCR write it (30%);
/// - a nasal with virama for ಂ before a stop of its group, as in ಸಮ್ಬನ್ಧ (30%);
/// - ೞ for ಳ and ಱ for ರ, as in Old Kannada text (5%);
/// - a nukta on ಜ and ಫ, as in loanwords (30%);
/// - ೀ, ೇ, ೊ and ೋ typed as 2 signs (30%);
/// - Kannada digits for ASCII digits (50% of numbers).
pub fn vary(text: &str, rng: &mut Rng) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut digits_kannada = None;
    for (i, &c) in chars.iter().enumerate() {
        let next = chars.get(i + 1).copied();
        match c {
            VIRAMA => {
                out.push(c);
                match next {
                    Some(n) if is_consonant(n) => {
                        if rng.chance(5) {
                            out.push(ZWNJ);
                        } else if rng.chance(5) {
                            out.push(ZWJ);
                        }
                    }
                    Some(n) if kannadi::chars::is_kannada(n) => {}
                    _ => out.push(ZWNJ),
                }
            }
            ANUSVARA => match next.and_then(group_nasal) {
                Some(nasal) if rng.chance(30) => {
                    out.push(nasal);
                    out.push(VIRAMA);
                }
                _ if next.is_some_and(kannadi::chars::is_kannada) && rng.chance(30) => {
                    out.push('೦')
                }
                _ => out.push(c),
            },
            'ಳ' if rng.chance(5) => out.push('\u{0CDE}'),
            'ರ' if rng.chance(5) && next != Some(VIRAMA) => out.push('\u{0CB1}'),
            'ಜ' | 'ಫ' if rng.chance(30) => {
                out.push(c);
                out.push(NUKTA);
            }
            '\u{0CC0}' if rng.chance(30) => out.push_str("\u{0CBF}\u{0CD5}"),
            '\u{0CC7}' if rng.chance(30) => out.push_str("\u{0CC6}\u{0CD5}"),
            '\u{0CCA}' if rng.chance(30) => out.push_str("\u{0CC6}\u{0CC2}"),
            '\u{0CCB}' if rng.chance(30) => out.push_str("\u{0CCA}\u{0CD5}"),
            '0'..='9' => {
                let kannada = *digits_kannada.get_or_insert_with(|| rng.chance(50));
                if kannada {
                    out.push(char::from_u32(0x0CE6 + (c as u32 - '0' as u32)).unwrap());
                } else {
                    out.push(c);
                }
            }
            _ => out.push(c),
        }
        if !c.is_ascii_digit() {
            digits_kannada = None;
        }
    }
    out
}
