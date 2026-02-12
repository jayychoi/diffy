//! Claude Code hook mode: stderr 피드백

use crate::model::{Diff, ReviewStatus};
use anyhow::Result;
use std::io::Write;

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

    writeln!(writer, "[diffy review result]")?;
    writeln!(writer, "rejected {} of {} hunks.\n", rejected.len(), total)?;

    for (file, hunk) in &rejected {
        writeln!(
            writer,
            "- {} (lines {}-{}): rejected",
            file.new_path,
            hunk.new_start,
            hunk.new_start + hunk.new_count.saturating_sub(1),
        )?;
        if let Some(comment) = &hunk.comment {
            writeln!(writer, "  comment: {}", comment)?;
        }
    }

    writeln!(writer, "\nplease fix the rejected hunks and try again.")?;
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
}
