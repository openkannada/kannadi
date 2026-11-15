//! Prints each feature on a sample sentence: `cargo run --example demo`.

use kannadi::{aksharas, modernise, normalize, numerals, transliterate, Scheme};

fn main() {
    let text = "ಕನ್ನಡ ನಾಡು ೧೯೫೬";
    for scheme in [Scheme::Iso15919, Scheme::Iast, Scheme::Baraha] {
        println!(
            "{scheme:>9}: {}",
            transliterate(text, Scheme::Kannada, scheme)
        );
    }
    let parts: Vec<&str> = aksharas("ಸ್ತ್ರೀಯರು").map(|a| a.text).collect();
    println!(" aksharas: {}", parts.join(" | "));
    println!("normalise: {}", normalize("ಬೆ೦ಗಳೂರು"));
    println!("modernise: {}", modernise("ಅಱಿವು ಬಾೞ್"));
    println!("  numbers: {}", numerals::format(2026));
}
