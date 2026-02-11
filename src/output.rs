//! accept된 헌크 재조립 및 JSON 출력

use std::io::Write;
use anyhow::Result;
use serde::Serialize;
use crate::model::{Diff, DiffLine, ReviewStatus};

/// accepted 헌크만 재조립하여 writer에 출력한다.
/// 하나 이상의 헌크가 출력되었으면 true, 아무것도 출력되지 않았으면 false를 반환한다.
pub fn write_diff<W: Write>(diff: &Diff, writer: &mut W) -> Result<bool> {
    let mut any_output = false;

    for file in &diff.files {
        // 바이너리 파일은 생략
        if file.is_binary {
            continue;
        }

        // accepted 헌크만 필터링
        let accepted_hunks: Vec<_> = file.hunks.iter()
            .filter(|h| h.status == ReviewStatus::Accepted)
            .collect();

        // 파일에 accepted 헌크가 없으면 생략
        if accepted_hunks.is_empty() {
            continue;
        }

        // 파일 헤더 출력
        writeln!(writer, "--- {}", file.raw_old_path)?;
        writeln!(writer, "+++ {}", file.raw_new_path)?;

        // 각 accepted 헌크 출력
        for hunk in accepted_hunks {
            writeln!(writer, "{}", hunk.header)?;

            for line in &hunk.lines {
                match line {
                    DiffLine::Context(s) => writeln!(writer, " {}", s)?,
                    DiffLine::Added(s) => writeln!(writer, "+{}", s)?,
                    DiffLine::Removed(s) => writeln!(writer, "-{}", s)?,
                    DiffLine::NoNewline => writeln!(writer, "\\ No newline at end of file")?,
                }
            }
        }

        any_output = true;
    }

    Ok(any_output)
}

/// JSON 출력용 구조체
#[derive(Serialize)]
struct JsonOutput<'a> {
    version: &'static str,
    summary: JsonSummary,
    files: Vec<JsonFile<'a>>,
}

#[derive(Serialize)]
struct JsonSummary {
    total_files: usize,
    total_hunks: usize,
    accepted: usize,
    rejected: usize,
    pending: usize,
}

#[derive(Serialize)]
struct JsonFile<'a> {
    path: &'a str,
    hunks: Vec<JsonHunk>,
}

#[derive(Serialize)]
struct JsonHunk {
    header: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    comment: Option<String>,
}

/// 리뷰 결과를 JSON으로 출력한다.
pub fn write_json<W: Write>(diff: &Diff, writer: &mut W) -> Result<()> {
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    let mut pending = 0usize;

    let files: Vec<JsonFile> = diff.files.iter().map(|f| {
        let hunks: Vec<JsonHunk> = f.hunks.iter().map(|h| {
            match h.status {
                ReviewStatus::Accepted => accepted += 1,
                ReviewStatus::Rejected => rejected += 1,
                ReviewStatus::Pending => pending += 1,
            }
            JsonHunk {
                header: h.header.clone(),
                status: format!("{:?}", h.status).to_lowercase(),
                comment: h.comment.clone(),
            }
        }).collect();
        JsonFile {
            path: &f.new_path,
            hunks,
        }
    }).collect();

    let output = JsonOutput {
        version: env!("CARGO_PKG_VERSION"),
        summary: JsonSummary {
            total_files: diff.files.len(),
            total_hunks: accepted + rejected + pending,
            accepted,
            rejected,
            pending,
        },
        files,
    };

    serde_json::to_writer_pretty(&mut *writer, &output)?;
    writeln!(writer)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FileDiff, Hunk};
    use indoc::indoc;
    use serde_json::Value;

    /// 테스트용 헌크 생성 헬퍼
    fn make_hunk(
        header: &str,
        old_start: u32,
        old_count: u32,
        new_start: u32,
        new_count: u32,
        lines: Vec<DiffLine>,
        status: ReviewStatus,
    ) -> Hunk {
        Hunk {
            header: header.to_string(),
            old_start,
            old_count,
            new_start,
            new_count,
            lines,
            status,
            comment: None,
        }
    }

    /// 테스트용 파일 생성 헬퍼
    fn make_file(old_path: &str, new_path: &str, hunks: Vec<Hunk>, is_binary: bool) -> FileDiff {
        FileDiff {
            old_path: old_path.trim_start_matches("a/").trim_start_matches("b/").to_string(),
            new_path: new_path.trim_start_matches("a/").trim_start_matches("b/").to_string(),
            raw_old_path: old_path.to_string(),
            raw_new_path: new_path.to_string(),
            hunks,
            is_binary,
        }
    }

    #[test]
    fn test_all_accepted() {
        let hunk = make_hunk(
            "@@ -1,3 +1,3 @@",
            1, 3, 1, 3,
            vec![
                DiffLine::Context("line1".to_string()),
                DiffLine::Removed("line2".to_string()),
                DiffLine::Added("line2 modified".to_string()),
                DiffLine::Context("line3".to_string()),
            ],
            ReviewStatus::Accepted,
        );

        let file = make_file("a/file.txt", "b/file.txt", vec![hunk], false);
        let diff = Diff { files: vec![file] };

        let mut output = Vec::new();
        let result = write_diff(&diff, &mut output).unwrap();

        assert!(result, "Should return true when hunks are written");

        let expected = indoc! {"
            --- a/file.txt
            +++ b/file.txt
            @@ -1,3 +1,3 @@
             line1
            -line2
            +line2 modified
             line3
        "};

        assert_eq!(String::from_utf8(output).unwrap(), expected);
    }

    #[test]
    fn test_all_rejected() {
        let hunk = make_hunk(
            "@@ -1,3 +1,3 @@",
            1, 3, 1, 3,
            vec![
                DiffLine::Context("line1".to_string()),
                DiffLine::Removed("line2".to_string()),
                DiffLine::Added("line2 modified".to_string()),
                DiffLine::Context("line3".to_string()),
            ],
            ReviewStatus::Rejected,
        );

        let file = make_file("a/file.txt", "b/file.txt", vec![hunk], false);
        let diff = Diff { files: vec![file] };

        let mut output = Vec::new();
        let result = write_diff(&diff, &mut output).unwrap();

        assert!(!result, "Should return false when no hunks are written");
        assert!(output.is_empty(), "Should produce empty output");
    }

    #[test]
    fn test_partial_accept() {
        let hunk1 = make_hunk(
            "@@ -1,2 +1,2 @@",
            1, 2, 1, 2,
            vec![
                DiffLine::Removed("old line".to_string()),
                DiffLine::Added("new line".to_string()),
            ],
            ReviewStatus::Accepted,
        );

        let hunk2 = make_hunk(
            "@@ -10,2 +10,2 @@",
            10, 2, 10, 2,
            vec![
                DiffLine::Removed("another old".to_string()),
                DiffLine::Added("another new".to_string()),
            ],
            ReviewStatus::Rejected,
        );

        let hunk3 = make_hunk(
            "@@ -20,2 +20,2 @@",
            20, 2, 20, 2,
            vec![
                DiffLine::Context("context line".to_string()),
                DiffLine::Added("added line".to_string()),
            ],
            ReviewStatus::Accepted,
        );

        let file = make_file("a/file.txt", "b/file.txt", vec![hunk1, hunk2, hunk3], false);
        let diff = Diff { files: vec![file] };

        let mut output = Vec::new();
        let result = write_diff(&diff, &mut output).unwrap();

        assert!(result, "Should return true when some hunks are written");

        let expected = indoc! {"
            --- a/file.txt
            +++ b/file.txt
            @@ -1,2 +1,2 @@
            -old line
            +new line
            @@ -20,2 +20,2 @@
             context line
            +added line
        "};

        assert_eq!(String::from_utf8(output).unwrap(), expected);
    }

    #[test]
    fn test_no_newline_roundtrip() {
        let hunk = make_hunk(
            "@@ -1,1 +1,1 @@",
            1, 1, 1, 1,
            vec![
                DiffLine::Removed("old content".to_string()),
                DiffLine::NoNewline,
                DiffLine::Added("new content".to_string()),
                DiffLine::NoNewline,
            ],
            ReviewStatus::Accepted,
        );

        let file = make_file("a/file.txt", "b/file.txt", vec![hunk], false);
        let diff = Diff { files: vec![file] };

        let mut output = Vec::new();
        let result = write_diff(&diff, &mut output).unwrap();

        assert!(result, "Should return true when hunks with NoNewline are written");

        let expected = indoc! {"
            --- a/file.txt
            +++ b/file.txt
            @@ -1,1 +1,1 @@
            -old content
            \\ No newline at end of file
            +new content
            \\ No newline at end of file
        "};

        assert_eq!(String::from_utf8(output).unwrap(), expected);
    }

    #[test]
    fn test_binary_file_skipped() {
        let hunk = make_hunk(
            "@@ -1,1 +1,1 @@",
            1, 1, 1, 1,
            vec![DiffLine::Added("binary content".to_string())],
            ReviewStatus::Accepted,
        );

        let file = make_file("a/image.png", "b/image.png", vec![hunk], true);
        let diff = Diff { files: vec![file] };

        let mut output = Vec::new();
        let result = write_diff(&diff, &mut output).unwrap();

        assert!(!result, "Should return false when binary files are skipped");
        assert!(output.is_empty(), "Should produce empty output for binary files");
    }

    #[test]
    fn test_mixed_files() {
        // 첫 번째 파일: accepted 헌크 있음
        let file1_hunk = make_hunk(
            "@@ -1,1 +1,1 @@",
            1, 1, 1, 1,
            vec![DiffLine::Added("file1 change".to_string())],
            ReviewStatus::Accepted,
        );
        let file1 = make_file("a/file1.txt", "b/file1.txt", vec![file1_hunk], false);

        // 두 번째 파일: 모두 rejected
        let file2_hunk = make_hunk(
            "@@ -1,1 +1,1 @@",
            1, 1, 1, 1,
            vec![DiffLine::Added("file2 change".to_string())],
            ReviewStatus::Rejected,
        );
        let file2 = make_file("a/file2.txt", "b/file2.txt", vec![file2_hunk], false);

        // 세 번째 파일: accepted 헌크 있음
        let file3_hunk = make_hunk(
            "@@ -1,1 +1,1 @@",
            1, 1, 1, 1,
            vec![DiffLine::Added("file3 change".to_string())],
            ReviewStatus::Accepted,
        );
        let file3 = make_file("a/file3.txt", "b/file3.txt", vec![file3_hunk], false);

        let diff = Diff { files: vec![file1, file2, file3] };

        let mut output = Vec::new();
        let result = write_diff(&diff, &mut output).unwrap();

        assert!(result, "Should return true when some files have accepted hunks");

        let expected = indoc! {"
            --- a/file1.txt
            +++ b/file1.txt
            @@ -1,1 +1,1 @@
            +file1 change
            --- a/file3.txt
            +++ b/file3.txt
            @@ -1,1 +1,1 @@
            +file3 change
        "};

        assert_eq!(String::from_utf8(output).unwrap(), expected);
    }

    // --- JSON output tests ---

    #[test]
    fn test_json_all_accepted() {
        let hunk = make_hunk(
            "@@ -1,2 +1,2 @@", 1, 2, 1, 2,
            vec![DiffLine::Removed("old".to_string()), DiffLine::Added("new".to_string())],
            ReviewStatus::Accepted,
        );
        let file = make_file("a/file.txt", "b/file.txt", vec![hunk], false);
        let diff = Diff { files: vec![file] };

        let mut buf = Vec::new();
        write_json(&diff, &mut buf).unwrap();
        let json: Value = serde_json::from_slice(&buf).unwrap();

        assert_eq!(json["version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(json["summary"]["accepted"], 1);
        assert_eq!(json["summary"]["rejected"], 0);
        assert_eq!(json["summary"]["total_hunks"], 1);
        assert_eq!(json["files"][0]["path"], "file.txt");
        assert_eq!(json["files"][0]["hunks"][0]["status"], "accepted");
    }

    #[test]
    fn test_json_all_rejected() {
        let hunk = make_hunk(
            "@@ -1,1 +1,1 @@", 1, 1, 1, 1,
            vec![DiffLine::Added("x".to_string())],
            ReviewStatus::Rejected,
        );
        let file = make_file("a/f.rs", "b/f.rs", vec![hunk], false);
        let diff = Diff { files: vec![file] };

        let mut buf = Vec::new();
        write_json(&diff, &mut buf).unwrap();
        let json: Value = serde_json::from_slice(&buf).unwrap();

        assert_eq!(json["summary"]["accepted"], 0);
        assert_eq!(json["summary"]["rejected"], 1);
        assert_eq!(json["files"][0]["hunks"][0]["status"], "rejected");
    }

    #[test]
    fn test_json_partial() {
        let h1 = make_hunk("@@ -1,1 +1,1 @@", 1, 1, 1, 1,
            vec![DiffLine::Added("a".to_string())], ReviewStatus::Accepted);
        let h2 = make_hunk("@@ -5,1 +5,1 @@", 5, 1, 5, 1,
            vec![DiffLine::Added("b".to_string())], ReviewStatus::Rejected);
        let file = make_file("a/mix.rs", "b/mix.rs", vec![h1, h2], false);
        let diff = Diff { files: vec![file] };

        let mut buf = Vec::new();
        write_json(&diff, &mut buf).unwrap();
        let json: Value = serde_json::from_slice(&buf).unwrap();

        assert_eq!(json["summary"]["accepted"], 1);
        assert_eq!(json["summary"]["rejected"], 1);
        assert_eq!(json["summary"]["total_hunks"], 2);
    }

    #[test]
    fn test_json_empty() {
        let diff = Diff { files: vec![] };

        let mut buf = Vec::new();
        write_json(&diff, &mut buf).unwrap();
        let json: Value = serde_json::from_slice(&buf).unwrap();

        assert_eq!(json["summary"]["total_files"], 0);
        assert_eq!(json["summary"]["total_hunks"], 0);
        assert_eq!(json["files"].as_array().unwrap().len(), 0);
    }
}
