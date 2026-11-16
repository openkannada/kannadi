//! Rewriting a document by replacing byte ranges, so untouched parts stay as they were.

use std::ops::Range;

/// `source` with each range replaced. Ranges must not overlap; an insertion at the
/// start of a replaced range goes before the replacement.
pub fn apply(source: &str, mut edits: Vec<(Range<usize>, String)>) -> String {
    edits.sort_by_key(|(r, _)| (r.start, r.end));
    let mut out = String::with_capacity(source.len());
    let mut last = 0;
    for (range, text) in edits {
        out.push_str(&source[last..range.start]);
        out.push_str(&text);
        last = range.end;
    }
    out.push_str(&source[last..]);
    out
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
    }
}
