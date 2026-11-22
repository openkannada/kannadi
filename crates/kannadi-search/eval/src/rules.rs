//! `rules`: writes data/suffixes.tsv from kannada-morph's noun paradigms.

use std::collections::BTreeMap;
use std::fmt::Write;

use kannada_morph::class::{Final, ALL_CLASSES};
use kannada_morph::noun::Declension;
use kannada_morph::rules::rules_for;
use kannada_morph::Class;

/// Classes whose endings the stemmer removes. ಆಕೆ-type pronouns (She) decline like ಮನೆ.
/// Doubled loanwords (ಬಸ್, ಬಸ್ಸ) are left out: their genitive is any geminate, so their
/// rules would cut ಹಬ್ಬ and ಅಮ್ಮ to ಹಬ್ and ಅಮ್.
fn wanted(class: Class) -> bool {
    matches!(class, Class::Noun(d) if d != Declension::She && d != Declension::LoanDoubled)
}

pub fn table(morph_rev: &str, predicates: bool) -> String {
    // (class, strip, add) -> finals.
    let mut rows: BTreeMap<(String, String, String), Vec<String>> = BTreeMap::new();
    for &class in ALL_CLASSES.iter().filter(|&&c| wanted(c)) {
        for rule in rules_for(class) {
            if rule.tags.contains('|') && !(predicates && is_copula(&rule.tags)) {
                continue;
            }
            for f in &rule.finals {
                let (add, fin) = match *f {
                    Final::Consonant => (rule.add.clone(), "C".to_string()),
                    // ಇದು-type stems are pronouns and verbal nouns, all in ದು; allowing
                    // any final ು would read ಮರಕ್ಕೆ as ಮರು + ಕ್ಕೆ.
                    Final::Char(c) if class == Class::Noun(Declension::This) => {
                        (rule.add.clone(), format!("ದ{c}"))
                    }
                    Final::Char(c) => (rule.add.clone(), c.to_string()),
                    Final::Doubled(_) => continue,
                };
                let finals = rows
                    .entry((class.flag().to_string(), rule.strip.clone(), add))
                    .or_default();
                if !finals.contains(&fin) {
                    finals.push(fin);
                }
            }
        }
    }
    let mut out = String::new();
    writeln!(
        out,
        "# Noun endings for kannadi-search's stemmer: stem - strip + add = form, for stems"
    )
    .unwrap();
    writeln!(out, "# ending in 1 of `finals` (C: a consonant with its inherent vowel; else the stem's last characters).").unwrap();
    writeln!(out, "# Generated from the noun paradigms of kannada-morph (github.com/openkannada/kannada-spell,").unwrap();
    writeln!(
        out,
        "# commit {morph_rev}) by crates/kannadi-search/eval: case, plural and clitic forms only."
    )
    .unwrap();
    writeln!(out, "class\tstrip\tadd\tfinals").unwrap();
    for ((class, strip, add), finals) in rows {
        writeln!(out, "{class}\t{strip}\t{add}\t{}", finals.join(",")).unwrap();
    }
    out
}

/// ಆಗು ("become, be") and ಇರು ("there is") written fused to a noun: ಸ್ಪಷ್ಟವಾಗಿ, ಮರವಾದ,
/// ಮನೆಯಲ್ಲಿದೆ. The quotative ಎನ್ನು (ಮರವೆಂದು) and ಅಲ್ಲ (ಮರವಲ್ಲ) are left as words.
fn is_copula(tags: &str) -> bool {
    tags.split('|')
        .skip(1)
        .all(|p| p.starts_with("ಆಗು+") || p.starts_with("ಇರು+"))
}
