//! `kannadi type`: ranked Kannada words for words typed in English letters.

use std::io::Write;

use anyhow::Result;
use kannadi_typing::Engine;

/// Writes the best `n` words for each input: 1 line per input, the input and
/// its words separated by tabs. With `scores`, 1 line per word instead, with its
/// score and whether it is a longer word the letters only start.
pub fn run(
    engine: &Engine,
    inputs: &[String],
    prev: Option<&str>,
    n: usize,
    scores: bool,
    out: &mut dyn Write,
) -> Result<()> {
    let weights = engine.model().weights();
    for input in inputs {
        let candidates = engine.candidates(input, prev);
        let best = candidates.iter().take(n);
        if scores {
            for c in best {
                let kind = if c.extra > 0 {
                    "completion"
                } else if c.known {
                    "word"
                } else {
                    "spelling"
                };
                writeln!(out, "{input}\t{}\t{:.2}\t{kind}", c.word, c.score(&weights))?;
            }
        } else {
            let words: Vec<&str> = best.map(|c| c.word.as_str()).collect();
            writeln!(out, "{input}\t{}", words.join("\t"))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_1_line_per_input_or_per_word() {
        let engine = Engine::bundled();
        let mut out = Vec::new();
        run(
            &engine,
            &["kannada".into(), "nanna".into()],
            None,
            3,
            false,
            &mut out,
        )
        .unwrap();
        let text = String::from_utf8(out).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("kannada\tಕನ್ನಡ\t"));
        assert!(lines[1].starts_with("nanna\tನನ್ನ\t"));
        let mut out = Vec::new();
        run(&engine, &["kannada".into()], None, 3, true, &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text.lines().count(), 3);
        assert!(text.starts_with("kannada\tಕನ್ನಡ\t"));
        assert!(text.lines().next().unwrap().ends_with("\tword"));
    }
}
