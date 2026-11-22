//! Postgres: a rules file for the `unaccent` extension that folds Kannada spelling
//! variants inside Postgres full-text search. See `examples/postgres/README.md`.
//!
//! `unaccent` replaces fixed strings, longest first, with no context. That covers the
//! context-free part of [`crate::fold`]: joiners and other invisible characters,
//! the nukta, Old Kannada letters, vowel signs typed in 2 parts, a nasal with virama
//! before its own consonant group, and Kannada digits. Replacements are not combined
//! beyond the cases written out (ಸನ್‌ತೋಷ is), so rarer mixes stay apart, such as a
//! nukta inside a 2-part vowel sign. A nasal cluster at the start of a word folds
//! too, where [`crate::fold`] keeps it. Rules that depend on context (೦ typed for ಂ),
//! stemming and legacy text need keys from [`crate::Analyzer`].

use std::fmt::Write;

use kannadi::chars::{ANUSVARA, NUKTA, VIRAMA};

use crate::fold::NASAL_GROUPS;

/// The rules, 1 per line: a string and its replacement separated by a tab, or a
/// string alone to delete it.
///
/// ```
/// let rules = kannadi_search::postgres::unaccent_rules();
/// assert!(rules.lines().any(|l| l == "ೞ\tಳ"));
/// ```
pub fn unaccent_rules() -> String {
    let mut out = String::new();
    let mut rule = |from: &str, to: &str| {
        if to.is_empty() {
            writeln!(out, "{from}").unwrap();
        } else {
            writeln!(out, "{from}\t{to}").unwrap();
        }
    };
    // ZWNJ, ZWJ, ZWSP, soft hyphen, word joiner and BOM: rule S2 of the profile.
    for c in [
        '\u{200C}', '\u{200D}', '\u{200B}', '\u{00AD}', '\u{2060}', '\u{FEFF}',
    ] {
        rule(&c.to_string(), "");
    }
    rule(&NUKTA.to_string(), "");
    // Rule S1: Old Kannada letters.
    for (old, new) in [
        ("\u{0CDE}", "ಳ"),
        ("\u{0CB1}", "ರ"),
        ("\u{0CDD}", "ನ್"),
        ("\u{0CF3}", "ಂ"),
        ("\u{0CDC}", "ಶ್ರೀ"),
    ] {
        rule(old, new);
    }
    // Rule C1: vowel signs typed in 2 parts (Unicode canonical decompositions).
    for (parts, sign) in [
        ("\u{0CBF}\u{0CD5}", "\u{0CC0}"),
        ("\u{0CC6}\u{0CD5}", "\u{0CC7}"),
        ("\u{0CC6}\u{0CD6}", "\u{0CC8}"),
        ("\u{0CC6}\u{0CC2}", "\u{0CCA}"),
        ("\u{0CCA}\u{0CD5}", "\u{0CCB}"),
        ("\u{0CC6}\u{0CC2}\u{0CD5}", "\u{0CCB}"),
    ] {
        rule(parts, sign);
    }
    // A nasal with virama before its group's stop, also with a nukta on the nasal or an
    // invisible character after the virama: unaccent makes 1 pass, so these come whole.
    let between = [
        "", "\u{200C}", "\u{200D}", "\u{200B}", "\u{00AD}", "\u{2060}", "\u{FEFF}",
    ];
    for (nasal, first, last) in NASAL_GROUPS {
        for stop in first..=last {
            for nukta in ["", "\u{0CBC}"] {
                for gap in between {
                    rule(
                        &format!("{nasal}{nukta}{VIRAMA}{gap}{stop}"),
                        &format!("{ANUSVARA}{stop}"),
                    );
                }
            }
        }
    }
    // Rule C4 for the anusvara, as typed, and next to a folded nasal (ಕಂಞ್ಚ).
    rule("\u{0C82}\u{0C82}", "\u{0C82}");
    rule("\u{0C82}\u{0C82}\u{0C82}", "\u{0C82}");
    for (nasal, first, last) in NASAL_GROUPS {
        for stop in first..=last {
            rule(
                &format!("{ANUSVARA}{nasal}{VIRAMA}{stop}"),
                &format!("{ANUSVARA}{stop}"),
            );
        }
    }
    for (i, digit) in ('೦'..='೯').enumerate() {
        rule(&digit.to_string(), &i.to_string());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fold::fold;

    /// Applies the rules as `unaccent` does: at each position the longest match wins.
    fn apply(rules: &str, text: &str) -> String {
        let pairs: Vec<(&str, &str)> = rules
            .lines()
            .map(|l| l.split_once('\t').unwrap_or((l, "")))
            .collect();
        let mut out = String::new();
        let mut rest = text;
        while let Some(c) = rest.chars().next() {
            match pairs
                .iter()
                .filter(|(from, _)| rest.starts_with(from))
                .max_by_key(|(from, _)| from.len())
            {
                Some((from, to)) => {
                    out.push_str(to);
                    rest = &rest[from.len()..];
                }
                None => {
                    out.push(c);
                    rest = &rest[c.len_utf8()..];
                }
            }
        }
        out
    }

    #[test]
    fn rules_agree_with_fold_on_context_free_variants() {
        let rules = unaccent_rules();
        for word in [
            "ಕನ್\u{200C}ನಡ",
            "ರಾಜ್\u{200D}ಕುಮಾರ್",
            "ಬಾೞ್",
            "ಜ಼ೀರೋ",
            "ಕಿ\u{0CD5}ರ್ತಿ",
            "ಸಮ್ಬನ್ಧ",
            "ಗಙ್ಗಾ",
            "೨೦೨೬",
            "ಕನ್ನಡ",
            "ಸನ್\u{200C}ತೋಷ",
            "ಸಮ್\u{00AD}ಬಂಧ",
            "ಸನ಼್ತೋಷ",
            "ಕಂಂಡ",
            "ಕಂಂಂಡ",
            "ಕಂಞ್ಚ",
        ] {
            assert_eq!(apply(&rules, word), fold(word), "{word}");
        }
    }

    #[test]
    fn the_example_rules_file_is_current() {
        assert_eq!(
            include_str!("../examples/postgres/kannada.rules"),
            unaccent_rules(),
            "rerun: cargo run -p kannadi-search --example postgres_rules > examples/postgres/kannada.rules"
        );
    }

    #[test]
    fn every_rule_has_1_or_2_fields_and_no_spaces() {
        for line in unaccent_rules().lines() {
            assert!(!line.contains(' '), "{line:?}");
            assert!(line.split('\t').count() <= 2, "{line:?}");
        }
    }
}
