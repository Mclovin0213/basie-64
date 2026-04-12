use similar::{ChangeTag, TextDiff};

#[derive(Debug, PartialEq, Clone)]
pub enum DiffKind {
    Added,
    Removed,
    Unchanged,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: DiffKind,
    pub line_a: Option<String>,
    pub line_b: Option<String>,
    pub num_a: Option<usize>,
    pub num_b: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct DiffResult {
    pub lines: Vec<DiffLine>,
    pub additions: usize,
    pub removals: usize,
    pub unchanged: usize,
}

/// Split `input` on the first `\n---\n` or `\n===\n` delimiter.
/// Returns `None` if no delimiter is found or either side is empty.
pub fn parse_diff_input(input: &str) -> Option<(String, String)> {
    for delimiter in &["\n---\n", "\n===\n"] {
        if let Some(pos) = input.find(delimiter) {
            let a = input[..pos].trim().to_string();
            let b = input[pos + delimiter.len()..].trim().to_string();
            if !a.is_empty() && !b.is_empty() {
                return Some((a, b));
            }
        }
    }
    None
}

/// Produce a side-by-side line diff of two text strings.
pub fn diff_text(a: &str, b: &str) -> DiffResult {
    let text_diff = TextDiff::from_lines(a, b);
    let mut lines = Vec::new();
    let mut num_a = 1usize;
    let mut num_b = 1usize;
    let mut additions = 0usize;
    let mut removals = 0usize;
    let mut unchanged = 0usize;

    for change in text_diff.iter_all_changes() {
        let line = change.value().trim_end_matches('\n').to_string();
        match change.tag() {
            ChangeTag::Equal => {
                lines.push(DiffLine {
                    kind: DiffKind::Unchanged,
                    line_a: Some(line.clone()),
                    line_b: Some(line),
                    num_a: Some(num_a),
                    num_b: Some(num_b),
                });
                num_a += 1;
                num_b += 1;
                unchanged += 1;
            }
            ChangeTag::Delete => {
                lines.push(DiffLine {
                    kind: DiffKind::Removed,
                    line_a: Some(line),
                    line_b: None,
                    num_a: Some(num_a),
                    num_b: None,
                });
                num_a += 1;
                removals += 1;
            }
            ChangeTag::Insert => {
                lines.push(DiffLine {
                    kind: DiffKind::Added,
                    line_a: None,
                    line_b: Some(line),
                    num_a: None,
                    num_b: Some(num_b),
                });
                num_b += 1;
                additions += 1;
            }
        }
    }

    DiffResult {
        lines,
        additions,
        removals,
        unchanged,
    }
}

/// Produce a side-by-side hex-dump diff of two byte slices.
pub fn diff_binary(a: &[u8], b: &[u8]) -> DiffResult {
    let a_str = to_hex_dump(a);
    let b_str = to_hex_dump(b);
    diff_text(&a_str, &b_str)
}

fn to_hex_dump(data: &[u8]) -> String {
    data.chunks(16)
        .enumerate()
        .map(|(i, chunk)| {
            let hex: Vec<String> = chunk.iter().map(|b| format!("{:02x}", b)).collect();
            format!("{:08x}  {}", i * 16, hex.join(" "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_diff_input ──────────────────────────────────────────────────────

    #[test]
    fn parse_splits_on_triple_dash_delimiter() {
        let input = "SGVsbG8=\n---\nV29ybGQ=";
        let result = parse_diff_input(input);
        assert_eq!(result, Some(("SGVsbG8=".into(), "V29ybGQ=".into())));
    }

    #[test]
    fn parse_splits_on_triple_equals_delimiter() {
        let input = "SGVsbG8=\n===\nV29ybGQ=";
        let result = parse_diff_input(input);
        assert_eq!(result, Some(("SGVsbG8=".into(), "V29ybGQ=".into())));
    }

    #[test]
    fn parse_returns_none_without_delimiter() {
        assert_eq!(parse_diff_input("SGVsbG8="), None);
    }

    #[test]
    fn parse_returns_none_when_side_is_empty() {
        // Delimiter at the very start → left side is empty
        assert_eq!(parse_diff_input("\n---\nV29ybGQ="), None);
        // Delimiter at the very end → right side is empty
        assert_eq!(parse_diff_input("SGVsbG8=\n---\n"), None);
    }

    #[test]
    fn parse_trims_whitespace_from_each_side() {
        let input = "  SGVsbG8=  \n---\n  V29ybGQ=  ";
        let result = parse_diff_input(input);
        assert_eq!(result, Some(("SGVsbG8=".into(), "V29ybGQ=".into())));
    }

    #[test]
    fn parse_multiline_content_before_delimiter() {
        let input = "line1\nline2\n---\nother1\nother2";
        let result = parse_diff_input(input);
        assert_eq!(
            result,
            Some(("line1\nline2".into(), "other1\nother2".into()))
        );
    }

    // ── diff_text ─────────────────────────────────────────────────────────────

    #[test]
    fn diff_identical_inputs_has_no_changes() {
        let result = diff_text("hello\nworld\n", "hello\nworld\n");
        assert_eq!(result.additions, 0);
        assert_eq!(result.removals, 0);
        assert_eq!(result.unchanged, 2);
    }

    #[test]
    fn diff_single_char_change_produces_one_add_one_remove() {
        let result = diff_text("hello\n", "helo\n");
        assert_eq!(result.additions, 1);
        assert_eq!(result.removals, 1);
        assert_eq!(result.unchanged, 0);
    }

    #[test]
    fn diff_extra_line_in_b_is_addition() {
        let result = diff_text("line1\n", "line1\nline2\n");
        assert_eq!(result.additions, 1);
        assert_eq!(result.removals, 0);
        assert_eq!(result.unchanged, 1);
    }

    #[test]
    fn diff_both_empty_has_no_lines() {
        let result = diff_text("", "");
        assert_eq!(result.additions, 0);
        assert_eq!(result.removals, 0);
        assert_eq!(result.unchanged, 0);
        assert!(result.lines.is_empty());
    }

    #[test]
    fn diff_removed_line_has_none_on_right_side() {
        let result = diff_text("only in a\n", "");
        assert_eq!(result.removals, 1);
        let removed = result.lines.iter().find(|l| l.kind == DiffKind::Removed);
        assert!(removed.is_some());
        assert!(removed.unwrap().line_b.is_none());
        assert!(removed.unwrap().line_a.is_some());
    }

    #[test]
    fn diff_added_line_has_none_on_left_side() {
        let result = diff_text("", "only in b\n");
        assert_eq!(result.additions, 1);
        let added = result.lines.iter().find(|l| l.kind == DiffKind::Added);
        assert!(added.is_some());
        assert!(added.unwrap().line_a.is_none());
        assert!(added.unwrap().line_b.is_some());
    }

    #[test]
    fn diff_line_numbers_are_sequential_per_side() {
        let result = diff_text("a\nb\nc\n", "a\nc\n");
        // Left side: lines 1, 2, 3 (a, b, c)
        // Right side: lines 1, 2 (a, c)
        let left_nums: Vec<_> = result.lines.iter().filter_map(|l| l.num_a).collect();
        let right_nums: Vec<_> = result.lines.iter().filter_map(|l| l.num_b).collect();
        assert_eq!(left_nums, vec![1, 2, 3]);
        assert_eq!(right_nums, vec![1, 2]);
    }

    // ── diff_binary ───────────────────────────────────────────────────────────

    #[test]
    fn diff_binary_identical_has_no_changes() {
        let data = b"hello world";
        let result = diff_binary(data, data);
        assert_eq!(result.additions, 0);
        assert_eq!(result.removals, 0);
    }

    #[test]
    fn diff_binary_different_bytes_produces_hex_lines() {
        let a = b"hello";
        let b = b"world";
        let result = diff_binary(a, b);
        // Different content → at least one add + one remove
        assert!(result.additions > 0 || result.removals > 0);
        // Lines contain hex content
        let any_hex = result
            .lines
            .iter()
            .filter_map(|l| l.line_a.as_deref().or(l.line_b.as_deref()))
            .any(|s| s.contains("68") || s.contains("77")); // 'h' = 0x68, 'w' = 0x77
        assert!(any_hex);
    }
}
