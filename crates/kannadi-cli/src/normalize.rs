//! `kannadi normalize`: Unicode Kannada text to a form of the normalization profile.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use kannadi::normalize::{normalize_counted, Counts, Form};

const WRITING: &str = "writing the normalized text";

/// True if `e` comes from writing to a pipe whose reader has gone, as with `| head`.
pub fn is_broken_pipe(e: &anyhow::Error) -> bool {
    e.root_cause()
        .downcast_ref::<std::io::Error>()
        .is_some_and(|io| io.kind() == std::io::ErrorKind::BrokenPipe)
}

/// What a run found.
pub struct Outcome {
    /// How many times each rule changed the text, over all inputs.
    pub counts: Counts,
    /// Inputs that were not already in the form.
    pub changed: Vec<String>,
}

/// Normalizes each input line by line, writing to `out` unless `check` is set.
/// An empty list or `-` reads standard input.
pub fn run(inputs: &[PathBuf], form: Form, check: bool, out: &mut dyn Write) -> Result<Outcome> {
    let stdin = [PathBuf::from("-")];
    let inputs = if inputs.is_empty() {
        &stdin[..]
    } else {
        inputs
    };
    let mut outcome = Outcome {
        counts: Counts::default(),
        changed: Vec::new(),
    };
    for input in inputs {
        let name = input.display().to_string();
        let reader: Box<dyn Read> = if input == Path::new("-") {
            Box::new(std::io::stdin().lock())
        } else {
            Box::new(std::fs::File::open(input).with_context(|| format!("opening {name}"))?)
        };
        let counts = normalize_stream(reader, &name, form, (!check).then_some(&mut *out))?;
        if counts.total() > 0 {
            outcome.changed.push(name);
        }
        outcome.counts.add(&counts);
    }
    out.flush().context(WRITING)?;
    Ok(outcome)
}

/// Normalizes 1 line at a time, so memory stays small for large files. No rule looks
/// past a line break, so this gives the same text as normalizing the whole input.
fn normalize_stream(
    reader: impl Read,
    name: &str,
    form: Form,
    mut out: Option<&mut dyn Write>,
) -> Result<Counts> {
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    let mut total = Counts::default();
    loop {
        line.clear();
        let read = reader.read_line(&mut line);
        if read.with_context(|| format!("reading {name}: not UTF-8 text, or unreadable"))? == 0 {
            return Ok(total);
        }
        let (text, counts) = normalize_counted(&line, form);
        total.add(&counts);
        if let Some(out) = out.as_deref_mut() {
            out.write_all(text.as_bytes()).context(WRITING)?;
        }
    }
}

/// The counts as TSV: rule id, rule name, count; rules of other forms are left out.
pub fn stats_tsv(counts: &Counts, form: Form) -> String {
    let mut tsv = String::from("rule\tname\tchanges\n");
    for (rule, n) in counts.iter().filter(|(r, _)| r.form() <= form) {
        tsv += &format!("{}\t{}\t{n}\n", rule.id(), rule.name());
    }
    tsv
}

#[cfg(test)]
mod tests {
    use super::*;
    use kannadi::normalize::Rule;

    #[test]
    fn lines_are_normalized_and_line_ends_kept() {
        let mut out = Vec::new();
        let counts = normalize_stream(
            "ಆಫ್\u{200C}\r\nಬೆ೦ಗಳೂರು\n".as_bytes(),
            "in",
            Form::Legacy,
            Some(&mut out),
        )
        .unwrap();
        assert_eq!(String::from_utf8(out).unwrap(), "ಆಫ್\r\nಬೆಂಗಳೂರು\n");
        assert_eq!(counts.get(Rule::StrayJoiner), 1);
        assert_eq!(counts.get(Rule::ZeroAsAnusvara), 1);
    }

    #[test]
    fn a_joiner_before_a_line_break_is_stray() {
        let counts =
            normalize_stream("ಆಫ್\u{200C}\nಕ".as_bytes(), "in", Form::Canonical, None).unwrap();
        assert_eq!(counts.get(Rule::StrayJoiner), 1);
    }

    #[test]
    fn invalid_utf8_is_an_error() {
        assert!(normalize_stream(&b"\xff\n"[..], "in", Form::Canonical, None).is_err());
    }

    struct ClosedPipe;

    impl Write for ClosedPipe {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_closed_pipe_is_a_write_error_the_command_can_ignore() {
        let e = normalize_stream(
            "ಕ\n".as_bytes(),
            "in",
            Form::Canonical,
            Some(&mut ClosedPipe),
        )
        .unwrap_err();
        assert!(format!("{e:#}").starts_with("writing"), "{e:#}");
        assert!(is_broken_pipe(&e));
        let bad = normalize_stream(&b"\xff\n"[..], "in", Form::Canonical, None).unwrap_err();
        assert!(format!("{bad:#}").starts_with("reading in"), "{bad:#}");
        assert!(!is_broken_pipe(&bad));
    }

    #[test]
    fn stats_list_only_the_rules_of_the_form() {
        let tsv = stats_tsv(&Counts::default(), Form::Canonical);
        assert!(tsv.contains("C8\tstray-joiner\t0\n"));
        assert!(!tsv.contains("L1"));
        assert_eq!(
            stats_tsv(&Counts::default(), Form::Search).lines().count(),
            15
        );
    }
}
