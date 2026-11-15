//! Kannada numerals (೦ to ೯).

const ZERO: u32 = 0x0CE6;

/// Converts one ASCII digit to a Kannada digit. Other characters are returned unchanged.
pub fn to_kannada_digit(c: char) -> char {
    match c.to_digit(10) {
        Some(d) if c.is_ascii_digit() => char::from_u32(ZERO + d).unwrap_or(c),
        _ => c,
    }
}

/// Converts one Kannada digit to an ASCII digit. Other characters are returned unchanged.
pub fn to_ascii_digit(c: char) -> char {
    match kannada_digit_value(c) {
        Some(d) => char::from_digit(d, 10).unwrap_or(c),
        None => c,
    }
}

/// The value of a Kannada digit, or `None`.
pub fn kannada_digit_value(c: char) -> Option<u32> {
    let v = (c as u32).checked_sub(ZERO)?;
    (v < 10).then_some(v)
}

/// Replaces ASCII digits with Kannada digits.
///
/// ```
/// assert_eq!(kannadi::numerals::to_kannada_digits("ಮೇ 2026"), "ಮೇ ೨೦೨೬");
/// ```
pub fn to_kannada_digits(text: &str) -> String {
    text.chars().map(to_kannada_digit).collect()
}

/// Replaces Kannada digits with ASCII digits.
///
/// ```
/// assert_eq!(kannadi::numerals::to_ascii_digits("ಪುಟ ೧೨"), "ಪುಟ 12");
/// ```
pub fn to_ascii_digits(text: &str) -> String {
    text.chars().map(to_ascii_digit).collect()
}

/// Writes a number in Kannada digits.
///
/// ```
/// assert_eq!(kannadi::numerals::format(1956), "೧೯೫೬");
/// ```
pub fn format(n: u64) -> String {
    to_kannada_digits(&n.to_string())
}

/// Parses a number written in Kannada digits, ASCII digits, or a mix.
/// Returns `None` for empty input, any other character, or overflow.
///
/// ```
/// assert_eq!(kannadi::numerals::parse("೧೯೫೬"), Some(1956));
/// assert_eq!(kannadi::numerals::parse("೧,೦೦೦"), None);
/// ```
pub fn parse(text: &str) -> Option<u64> {
    if text.is_empty() {
        return None;
    }
    text.chars().try_fold(0u64, |acc, c| {
        let d =
            kannada_digit_value(c).or_else(|| c.is_ascii_digit().then(|| c as u32 - '0' as u32))?;
        acc.checked_mul(10)?.checked_add(u64::from(d))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digit_mapping_both_ways() {
        let ascii = "0123456789";
        let kn = "೦೧೨೩೪೫೬೭೮೯";
        assert_eq!(to_kannada_digits(ascii), kn);
        assert_eq!(to_ascii_digits(kn), ascii);
    }

    #[test]
    fn non_digits_untouched() {
        assert_eq!(to_kannada_digits("ಕ x ٣"), "ಕ x ٣");
        assert_eq!(to_ascii_digits("ಕ x ३"), "ಕ x ३");
    }

    #[test]
    fn parse_and_format() {
        assert_eq!(parse("0"), Some(0));
        assert_eq!(parse("೨೦26"), Some(2026));
        assert_eq!(parse(""), None);
        assert_eq!(parse("೧a"), None);
        assert_eq!(parse("99999999999999999999"), None);
        assert_eq!(format(0), "೦");
        assert_eq!(parse(&format(u64::MAX)), Some(u64::MAX));
    }
}
