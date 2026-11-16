//! Rewriting a document by replacing byte ranges, so untouched parts stay as they were.

use std::ops::Range;

/// `source` with each range replaced. An insertion at the start of a replaced
/// range goes before the replacement. An edit that overlaps an earlier one is
/// left out; check with [`overlap`] first where that can happen.
pub fn apply(source: &str, mut edits: Vec<(Range<usize>, String)>) -> String {
    edits.sort_by_key(|(r, _)| (r.start, r.end));
    let mut out = String::with_capacity(source.len());
    let mut last = 0;
    for (range, text) in edits {
        if range.start < last {
            continue;
        }
        out.push_str(&source[last..range.start]);
        out.push_str(&text);
        last = range.end;
    }
    out.push_str(&source[last..]);
    out
}

/// Whether any 2 edits overlap.
pub fn overlap(edits: &[(Range<usize>, String)]) -> bool {
    let mut ranges: Vec<&Range<usize>> = edits.iter().map(|(r, _)| r).collect();
    ranges.sort_by_key(|r| (r.start, r.end));
    ranges.windows(2).any(|w| w[1].start < w[0].end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edits_apply_in_any_order() {
        let edits = vec![
            (4..5, "X".to_string()),
            (0..1, "".to_string()),
            (2..2, "+".to_string()),
        ];
        assert_eq!(apply("abcdef", edits), "b+cdXf");
        let same_start = vec![(1..3, "R".to_string()), (1..1, "I".to_string())];
        assert_eq!(apply("abcd", same_start), "aIRd");
        let overlapping = vec![(0..3, "X".to_string()), (2..4, "Y".to_string())];
        assert!(overlap(&overlapping));
        assert_eq!(apply("abcd", overlapping), "Xd");
    }
}
