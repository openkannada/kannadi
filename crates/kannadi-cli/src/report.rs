//! The per-file report: 1 row per problem, written next to the converted file.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::convert::{Issue, Kind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub location: String,
    pub kind: Kind,
    pub detail: String,
    pub context: String,
}

impl Row {
    pub fn new(location: String, issue: &Issue) -> Self {
        Row {
            location,
            kind: issue.kind,
            detail: issue.detail.clone(),
            context: issue.context.clone(),
        }
    }
}

#[derive(Debug, Default)]
pub struct Report {
    pub rows: Vec<Row>,
}

impl Report {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// True if any row is a problem, as [`Kind::is_problem`] says.
    pub fn has_problems(&self) -> bool {
        self.rows.iter().any(|r| r.kind.is_problem())
    }

    pub fn count(&self, kind: Kind) -> usize {
        self.rows.iter().filter(|r| r.kind == kind).count()
    }

    /// Counts by kind, such as "2 unknown glyphs, 1 malformed syllable".
    pub fn summary(&self) -> String {
        [
            Kind::UnknownGlyph,
            Kind::Malformed,
            Kind::Unencodable,
            Kind::NotConverted,
            Kind::LikelyTypo,
        ]
        .into_iter()
        .map(|k| (k, self.count(k)))
        .filter(|&(_, n)| n > 0)
        .map(|(k, n)| format!("{n} {}", if n == 1 { k.label() } else { k.plural() }))
        .collect::<Vec<_>>()
        .join(", ")
    }

    pub fn to_tsv(&self) -> String {
        let mut out = String::from("location\tkind\tdetail\tcontext\n");
        for r in &self.rows {
            let _ = writeln!(
                out,
                "{}\t{}\t{}\t{}",
                clean(&r.location),
                r.kind.label(),
                clean(&r.detail),
                clean(&r.context)
            );
        }
        out
    }
}

/// Where the report for `output` goes: the same path with `.report.tsv` added.
pub fn path_for(output: &Path) -> PathBuf {
    let mut name = output.file_name().unwrap_or_default().to_os_string();
    name.push(".report.tsv");
    output.with_file_name(name)
}

/// Keeps every field on 1 line of the TSV, with no control characters.
fn clean(field: &str) -> String {
    field
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsv_keeps_rows_on_1_line() {
        let report = Report {
            rows: vec![Row {
                location: "line 2".into(),
                kind: Kind::Malformed,
                detail: "ಾ".into(),
                context: "a\tb\nc\0".into(),
            }],
        };
        assert_eq!(
            report.to_tsv(),
            "location\tkind\tdetail\tcontext\nline 2\tmalformed syllable\tಾ\ta b c \n"
        );
        assert_eq!(report.summary(), "1 malformed syllable");
    }

    #[test]
    fn likely_typos_are_counted_but_are_not_problems() {
        let row = |kind| Row {
            location: "line 1".into(),
            kind,
            detail: String::new(),
            context: String::new(),
        };
        let mut report = Report {
            rows: vec![row(Kind::LikelyTypo), row(Kind::LikelyTypo)],
        };
        assert!(!report.has_problems());
        assert_eq!(report.summary(), "2 likely typos");
        report.rows.insert(0, row(Kind::Malformed));
        assert!(report.has_problems());
        assert_eq!(report.summary(), "1 malformed syllable, 2 likely typos");
    }

    #[test]
    fn report_path_adds_a_suffix() {
        assert_eq!(
            path_for(Path::new("out/a.txt")),
            PathBuf::from("out/a.txt.report.tsv")
        );
    }
}
