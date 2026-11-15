//! The per-file report: 1 row per problem, written next to the converted file.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use crate::convert::Kind;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub location: String,
    pub kind: Kind,
    pub detail: String,
    pub context: String,
}

#[derive(Debug, Default)]
pub struct Report {
    pub rows: Vec<Row>,
}

impl Report {
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn count(&self, kind: Kind) -> usize {
        self.rows.iter().filter(|r| r.kind == kind).count()
    }

    /// Counts by kind, such as "2 unknown glyph, 1 malformed syllable".
    pub fn summary(&self) -> String {
        [Kind::UnknownGlyph, Kind::Malformed, Kind::Unencodable]
            .into_iter()
            .map(|k| (k, self.count(k)))
            .filter(|&(_, n)| n > 0)
            .map(|(k, n)| format!("{n} {}", k.label()))
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

/// Keeps every field on 1 line of the TSV.
fn clean(field: &str) -> String {
    field
        .chars()
        .map(|c| {
            if c == '\t' || c == '\n' || c == '\r' {
                ' '
            } else {
                c
            }
        })
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
                context: "a\tb\nc".into(),
            }],
        };
        assert_eq!(
            report.to_tsv(),
            "location\tkind\tdetail\tcontext\nline 2\tmalformed syllable\tಾ\ta b c\n"
        );
        assert_eq!(report.summary(), "1 malformed syllable");
    }

    #[test]
    fn report_path_adds_a_suffix() {
        assert_eq!(
            path_for(Path::new("out/a.txt")),
            PathBuf::from("out/a.txt.report.tsv")
        );
    }
}
