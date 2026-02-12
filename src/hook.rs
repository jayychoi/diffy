//! Claude Code hook mode: stderr 피드백

use crate::model::{Diff, DiffLine, ReviewStatus};
use anyhow::Result;
use std::io::Write;

/// 피드백 최대 크기 (바이트)
fn feedback_max_size() -> usize {
    std::env::var("DIFFY_FEEDBACK_MAX_SIZE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(10_240)
}

/// DiffLine을 diff 형식 문자열로 변환
fn format_diff_lines(lines: &[DiffLine]) -> String {
    let mut result = String::new();
    for line in lines {
        match line {
            DiffLine::Context(s) => {
                result.push(' ');
                result.push_str(s);
                result.push('\n');
            }
            DiffLine::Added(s) => {
                result.push('+');
                result.push_str(s);
                result.push('\n');
            }
            DiffLine::Removed(s) => {
                result.push('-');
                result.push_str(s);
                result.push('\n');
            }
            DiffLine::NoNewline => {
                // skip NoNewline markers in feedback
            }
        }
    }
    result
}

/// 리뷰 결과를 stderr로 출력한다.
/// 모든 헌크가 accepted이면 true, rejected가 있으면 false를 반환한다.
pub fn write_feedback(diff: &Diff, writer: &mut impl Write) -> Result<bool> {
    let total: usize = diff.files.iter().map(|f| f.hunks.len()).sum();

    let rejected: Vec<_> = diff
        .files
        .iter()
        .flat_map(|f| f.hunks.iter().map(move |h| (f, h)))
        .filter(|(_, h)| h.status == ReviewStatus::Rejected)
        .collect();

    if rejected.is_empty() {
        writeln!(writer, "[diffy] all {} hunks accepted.", total)?;
        return Ok(true);
    }

    let max_size = feedback_max_size();
    let mut buffer = Vec::new();
    let mut truncated = false;

    writeln!(&mut buffer, "[diffy review result]")?;
    writeln!(
        &mut buffer,
        "rejected {} of {} hunks.\n",
        rejected.len(),
        total
    )?;

    for (file, hunk) in &rejected {
        let mut hunk_buffer = Vec::new();

        writeln!(
            &mut hunk_buffer,
            "- {} (lines {}-{}): rejected",
            file.new_path,
            hunk.new_start,
            hunk.new_start + hunk.new_count.saturating_sub(1),
        )?;

        if let Some(comment) = &hunk.comment {
            writeln!(&mut hunk_buffer, "  comment: {}", comment)?;
        }

        // Add diff code block
        let diff_content = format_diff_lines(&hunk.lines);
        if !diff_content.is_empty() {
            writeln!(&mut hunk_buffer, "  ```diff")?;
            write!(&mut hunk_buffer, "{}", diff_content)?;
            writeln!(&mut hunk_buffer, "  ```")?;
        }

        writeln!(&mut hunk_buffer)?;

        // Check if adding this hunk would exceed max size
        if buffer.len() + hunk_buffer.len() > max_size {
            truncated = true;
            break;
        }

        buffer.extend_from_slice(&hunk_buffer);
    }

    if truncated {
        writeln!(&mut buffer, "... (output truncated)")?;
    }

    writeln!(&mut buffer, "please fix the rejected hunks and try again.")?;

    writer.write_all(&buffer)?;
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Diff, DiffLine, FileDiff, Hunk};

    fn make_hunk(new_start: u32, new_count: u32, status: ReviewStatus) -> Hunk {
        Hunk {
            header: format!("@@ -1,1 +{},{} @@", new_start, new_count),
            old_start: 1,
            old_count: 1,
            new_start,
            new_count,
            lines: vec![DiffLine::Added("test".to_string())],
            status,
            comment: None,
        }
    }

    fn make_file(path: &str, hunks: Vec<Hunk>) -> FileDiff {
        FileDiff {
            old_path: path.to_string(),
            new_path: path.to_string(),
            raw_old_path: format!("a/{}", path),
            raw_new_path: format!("b/{}", path),
            hunks,
            is_binary: false,
        }
    }

    #[test]
    fn test_all_accepted() {
        let diff = Diff {
            files: vec![make_file(
                "src/main.rs",
                vec![
                    make_hunk(1, 3, ReviewStatus::Accepted),
                    make_hunk(10, 5, ReviewStatus::Accepted),
                ],
            )],
        };

        let mut output = Vec::new();
        let result = write_feedback(&diff, &mut output).unwrap();
        assert!(result);

        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("all 2 hunks accepted"));
    }

    #[test]
    fn test_some_rejected() {
        let diff = Diff {
            files: vec![make_file(
                "src/main.rs",
                vec![
                    make_hunk(1, 3, ReviewStatus::Accepted),
                    make_hunk(10, 5, ReviewStatus::Rejected),
                ],
            )],
        };

        let mut output = Vec::new();
        let result = write_feedback(&diff, &mut output).unwrap();
        assert!(!result);

        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("rejected 1 of 2 hunks"));
        assert!(text.contains("src/main.rs"));
    }

    #[test]
    fn test_all_rejected() {
        let diff = Diff {
            files: vec![make_file(
                "src/lib.rs",
                vec![
                    make_hunk(1, 2, ReviewStatus::Rejected),
                    make_hunk(5, 3, ReviewStatus::Rejected),
                ],
            )],
        };

        let mut output = Vec::new();
        let result = write_feedback(&diff, &mut output).unwrap();
        assert!(!result);

        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("rejected 2 of 2 hunks"));
        assert!(text.contains("please fix"));
    }

    #[test]
    fn test_feedback_includes_diff_context() {
        let mut hunk = make_hunk(45, 8, ReviewStatus::Rejected);
        hunk.lines = vec![
            DiffLine::Context("fn main() {".to_string()),
            DiffLine::Removed("    old_code();".to_string()),
            DiffLine::Added("    new_code();".to_string()),
            DiffLine::Context("}".to_string()),
        ];

        let diff = Diff {
            files: vec![make_file("src/main.rs", vec![hunk])],
        };

        let mut output = Vec::new();
        let result = write_feedback(&diff, &mut output).unwrap();
        assert!(!result);

        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("rejected 1 of 1 hunks"));
        assert!(text.contains("```diff"));
        assert!(text.contains(" fn main() {"));
        assert!(text.contains("-    old_code();"));
        assert!(text.contains("+    new_code();"));
        assert!(text.contains(" }"));
    }

    #[test]
    fn test_feedback_with_comment_and_diff() {
        let mut hunk = make_hunk(12, 3, ReviewStatus::Rejected);
        hunk.comment = Some("this breaks error handling".to_string());
        hunk.lines = vec![
            DiffLine::Removed("old line".to_string()),
            DiffLine::Added("new line".to_string()),
        ];

        let diff = Diff {
            files: vec![make_file("src/lib.rs", vec![hunk])],
        };

        let mut output = Vec::new();
        let result = write_feedback(&diff, &mut output).unwrap();
        assert!(!result);

        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("comment: this breaks error handling"));
        assert!(text.contains("```diff"));
        assert!(text.contains("-old line"));
        assert!(text.contains("+new line"));
    }

    #[test]
    fn test_feedback_truncation() {
        // Create many rejected hunks with long lines to exceed 10KB
        let mut hunks = Vec::new();
        for i in 0..100 {
            let mut hunk = make_hunk(i * 10, 50, ReviewStatus::Rejected);
            hunk.lines = vec![
                DiffLine::Context("context line with some text".to_string()),
                DiffLine::Removed(format!(
                    "old code line {} with lots of text to make it longer {}",
                    i,
                    "x".repeat(200)
                )),
                DiffLine::Added(format!(
                    "new code line {} with lots of text to make it longer {}",
                    i,
                    "y".repeat(200)
                )),
                DiffLine::Context("more context".to_string()),
            ];
            hunks.push(hunk);
        }

        let diff = Diff {
            files: vec![make_file("src/large.rs", hunks)],
        };

        let mut output = Vec::new();
        let result = write_feedback(&diff, &mut output).unwrap();
        assert!(!result);

        let text = String::from_utf8(output).unwrap();

        // Should be truncated
        assert!(text.contains("... (output truncated)"));
        assert!(text.contains("please fix"));

        // Output should respect size limit
        assert!(text.len() <= feedback_max_size() + 200); // small buffer for final message
    }
}
