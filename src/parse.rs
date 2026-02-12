//! unified diff 파서

use crate::model::{Diff, DiffLine, FileDiff, Hunk, ReviewStatus};
use anyhow::Result;

/// unified diff 텍스트를 파싱한다
pub fn parse_diff(input: &str) -> Result<Diff> {
    let mut files = Vec::new();
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        // diff --git 라인을 찾거나 --- 라인 찾기
        if lines[i].starts_with("diff --git ") {
            i += 1; // diff --git 라인 건너뛰기

            // --- 라인 찾기
            while i < lines.len()
                && !lines[i].starts_with("---")
                && !lines[i].starts_with("Binary files")
            {
                i += 1;
            }

            if i >= lines.len() {
                break;
            }
        }

        // 바이너리 파일 체크
        if lines[i].starts_with("Binary files") {
            if let Some(file_diff) = parse_binary_file(lines[i]) {
                files.push(file_diff);
            }
            i += 1;
            continue;
        }

        // --- 라인 찾기
        if !lines[i].starts_with("---") {
            i += 1;
            continue;
        }

        let raw_old_path = lines[i].trim_start_matches("---").trim().to_string();
        let old_path = parse_path_from_header(lines[i]);
        i += 1;

        if i >= lines.len() {
            break;
        }

        // +++ 라인 찾기
        if !lines[i].starts_with("+++") {
            continue;
        }

        let raw_new_path = lines[i].trim_start_matches("+++").trim().to_string();
        let new_path = parse_path_from_header(lines[i]);
        i += 1;

        // 헌크들 파싱
        let mut hunks = Vec::new();
        while i < lines.len() && lines[i].starts_with("@@") {
            let (hunk, next_i) = parse_hunk(&lines, i);
            hunks.push(hunk);
            i = next_i;
        }

        files.push(FileDiff {
            old_path,
            new_path,
            raw_old_path,
            raw_new_path,
            hunks,
            is_binary: false,
        });
    }

    Ok(Diff { files })
}

/// 바이너리 파일 라인 파싱
fn parse_binary_file(line: &str) -> Option<FileDiff> {
    // "Binary files a/path and b/path differ" 형태
    let parts: Vec<&str> = line.split_whitespace().collect();
    if parts.len() < 5 {
        return None;
    }

    let raw_old_path = parts[2].to_string();
    let raw_new_path = parts[4].to_string();
    let old_path = parts[2].trim_start_matches("a/").to_string();
    let new_path = parts[4].trim_start_matches("b/").to_string();

    Some(FileDiff {
        old_path,
        new_path,
        raw_old_path,
        raw_new_path,
        hunks: Vec::new(),
        is_binary: true,
    })
}

/// --- 또는 +++ 라인에서 경로 추출
fn parse_path_from_header(line: &str) -> String {
    let path = if line.starts_with("---") {
        line.trim_start_matches("---").trim()
    } else if line.starts_with("+++") {
        line.trim_start_matches("+++").trim()
    } else {
        return String::new();
    };

    // /dev/null 체크
    if path == "/dev/null" {
        return path.to_string();
    }

    // a/ 또는 b/ 접두사 제거
    path.trim_start_matches("a/")
        .trim_start_matches("b/")
        .to_string()
}

/// 헌크 하나 파싱
fn parse_hunk(lines: &[&str], start: usize) -> (Hunk, usize) {
    let header = lines[start].to_string();
    let (old_start, old_count, new_start, new_count) = parse_hunk_header(&header);

    let mut hunk_lines = Vec::new();
    let mut i = start + 1;

    while i < lines.len() {
        let line = lines[i];

        // 다음 헌크나 파일로 진입
        if line.starts_with("@@") || line.starts_with("diff --git") || line.starts_with("---") {
            break;
        }

        // No newline 마커
        if line.starts_with("\\ No newline") {
            hunk_lines.push(DiffLine::NoNewline);
            i += 1;
            continue;
        }

        // 일반 diff 라인
        if line.is_empty() {
            hunk_lines.push(DiffLine::Context(String::new()));
            i += 1;
            continue;
        }

        let first_char = line.chars().next().unwrap_or(' ');
        match first_char {
            ' ' => {
                hunk_lines.push(DiffLine::Context(line[1..].to_string()));
            }
            '+' => {
                hunk_lines.push(DiffLine::Added(line[1..].to_string()));
            }
            '-' => {
                hunk_lines.push(DiffLine::Removed(line[1..].to_string()));
            }
            _ => {
                // 알 수 없는 라인은 무시 (permissive)
            }
        }

        i += 1;
    }

    let hunk = Hunk {
        header,
        old_start,
        old_count,
        new_start,
        new_count,
        lines: hunk_lines,
        status: ReviewStatus::Pending,
        comment: None,
    };

    (hunk, i)
}

/// @@ -a,b +c,d @@ 헤더 파싱
fn parse_hunk_header(header: &str) -> (u32, u32, u32, u32) {
    // @@ -1,3 +1,4 @@ 형태에서 숫자 추출
    let parts: Vec<&str> = header.split_whitespace().collect();
    if parts.len() < 3 {
        return (0, 0, 0, 0);
    }

    let old_part = parts[1].trim_start_matches('-');
    let new_part = parts[2].trim_start_matches('+');

    let (old_start, old_count) = parse_range(old_part);
    let (new_start, new_count) = parse_range(new_part);

    (old_start, old_count, new_start, new_count)
}

/// "a,b" 또는 "a" 형태 파싱
fn parse_range(range: &str) -> (u32, u32) {
    if let Some(comma_pos) = range.find(',') {
        let start = range[..comma_pos].parse().unwrap_or(0);
        let count = range[comma_pos + 1..].parse().unwrap_or(0);
        (start, count)
    } else {
        let start = range.parse().unwrap_or(0);
        (start, 1) // count 생략 시 1로 간주
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn test_empty_input() {
        let diff = parse_diff("").unwrap();
        assert_eq!(diff.files.len(), 0);
    }

    #[test]
    fn test_single_hunk() {
        let input = indoc! {"
            --- a/file.txt
            +++ b/file.txt
            @@ -1,3 +1,4 @@
             line1
             line2
            +added line
             line3
        "};

        let diff = parse_diff(input).unwrap();
        assert_eq!(diff.files.len(), 1);

        let file = &diff.files[0];
        assert_eq!(file.old_path, "file.txt");
        assert_eq!(file.new_path, "file.txt");
        assert_eq!(file.is_binary, false);
        assert_eq!(file.hunks.len(), 1);

        let hunk = &file.hunks[0];
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.old_count, 3);
        assert_eq!(hunk.new_start, 1);
        assert_eq!(hunk.new_count, 4);
        assert_eq!(hunk.lines.len(), 4);
        assert!(matches!(hunk.lines[0], DiffLine::Context(_)));
        assert!(matches!(hunk.lines[1], DiffLine::Context(_)));
        assert!(matches!(hunk.lines[2], DiffLine::Added(_)));
        assert!(matches!(hunk.lines[3], DiffLine::Context(_)));
        assert_eq!(hunk.status, ReviewStatus::Pending);
    }

    #[test]
    fn test_multi_hunk() {
        let input = indoc! {"
            --- a/file.txt
            +++ b/file.txt
            @@ -1,2 +1,3 @@
             line1
            +added line
             line2
            @@ -10,2 +11,2 @@
            -removed line
             line10
        "};

        let diff = parse_diff(input).unwrap();
        assert_eq!(diff.files.len(), 1);

        let file = &diff.files[0];
        assert_eq!(file.hunks.len(), 2);

        let hunk1 = &file.hunks[0];
        assert_eq!(hunk1.old_start, 1);
        assert_eq!(hunk1.lines.len(), 3);

        let hunk2 = &file.hunks[1];
        assert_eq!(hunk2.old_start, 10);
        assert_eq!(hunk2.lines.len(), 2);
        assert!(matches!(hunk2.lines[0], DiffLine::Removed(_)));
    }

    #[test]
    fn test_multi_file() {
        let input = indoc! {"
            diff --git a/file1.txt b/file1.txt
            --- a/file1.txt
            +++ b/file1.txt
            @@ -1,1 +1,2 @@
             line1
            +added to file1
            diff --git a/file2.txt b/file2.txt
            --- a/file2.txt
            +++ b/file2.txt
            @@ -1,1 +1,2 @@
             line1
            +added to file2
        "};

        let diff = parse_diff(input).unwrap();
        assert_eq!(diff.files.len(), 2);

        assert_eq!(diff.files[0].old_path, "file1.txt");
        assert_eq!(diff.files[1].old_path, "file2.txt");
        assert_eq!(diff.files[0].hunks.len(), 1);
        assert_eq!(diff.files[1].hunks.len(), 1);
    }

    #[test]
    fn test_invalid_input() {
        let input = indoc! {"
            this is not a valid diff
            random text
            @@ invalid header
        "};

        let diff = parse_diff(input).unwrap();
        // permissive parsing - 빈 결과 반환
        assert_eq!(diff.files.len(), 0);
    }

    #[test]
    fn test_binary_file() {
        let input = indoc! {"
            diff --git a/image.png b/image.png
            Binary files a/image.png and b/image.png differ
        "};

        let diff = parse_diff(input).unwrap();
        assert_eq!(diff.files.len(), 1);

        let file = &diff.files[0];
        assert_eq!(file.old_path, "image.png");
        assert_eq!(file.new_path, "image.png");
        assert_eq!(file.is_binary, true);
        assert_eq!(file.hunks.len(), 0);
    }

    #[test]
    fn test_new_deleted_file() {
        let input_new = indoc! {"
            --- /dev/null
            +++ b/newfile.txt
            @@ -0,0 +1,2 @@
            +new line 1
            +new line 2
        "};

        let diff = parse_diff(input_new).unwrap();
        assert_eq!(diff.files.len(), 1);
        assert_eq!(diff.files[0].old_path, "/dev/null");
        assert_eq!(diff.files[0].new_path, "newfile.txt");

        let input_deleted = indoc! {"
            --- a/oldfile.txt
            +++ /dev/null
            @@ -1,2 +0,0 @@
            -deleted line 1
            -deleted line 2
        "};

        let diff = parse_diff(input_deleted).unwrap();
        assert_eq!(diff.files.len(), 1);
        assert_eq!(diff.files[0].old_path, "oldfile.txt");
        assert_eq!(diff.files[0].new_path, "/dev/null");
    }

    #[test]
    fn test_no_newline_marker() {
        let input = indoc! {"
            --- a/file.txt
            +++ b/file.txt
            @@ -1,2 +1,2 @@
             line1
            -line2
            \\ No newline at end of file
            +line2
        "};

        let diff = parse_diff(input).unwrap();
        assert_eq!(diff.files.len(), 1);

        let hunk = &diff.files[0].hunks[0];
        assert_eq!(hunk.lines.len(), 4);
        assert!(matches!(hunk.lines[0], DiffLine::Context(_)));
        assert!(matches!(hunk.lines[1], DiffLine::Removed(_)));
        assert!(matches!(hunk.lines[2], DiffLine::NoNewline));
        assert!(matches!(hunk.lines[3], DiffLine::Added(_)));
    }

    #[test]
    fn test_raw_path_prefix_preserved() {
        let input = indoc! {"
            --- a/src/main.rs
            +++ b/src/main.rs
            @@ -1,3 +1,4 @@
             line1
             line2
            +added line
             line3
        "};

        let diff = parse_diff(input).unwrap();
        assert_eq!(diff.files.len(), 1);

        let file = &diff.files[0];
        // 표시용 경로는 접두사 제거됨
        assert_eq!(file.old_path, "src/main.rs");
        assert_eq!(file.new_path, "src/main.rs");
        // 원본 경로는 접두사 보존됨
        assert_eq!(file.raw_old_path, "a/src/main.rs");
        assert_eq!(file.raw_new_path, "b/src/main.rs");
    }

    #[test]
    fn test_empty_context_line() {
        let input = indoc! {"
            --- a/file.txt
            +++ b/file.txt
            @@ -1,3 +1,3 @@
             line1

             line3
        "};

        let diff = parse_diff(input).unwrap();
        assert_eq!(diff.files.len(), 1);

        let hunk = &diff.files[0].hunks[0];
        assert_eq!(hunk.lines.len(), 3);
        assert!(matches!(hunk.lines[0], DiffLine::Context(_)));
        assert!(matches!(hunk.lines[1], DiffLine::Context(ref s) if s.is_empty()));
        assert!(matches!(hunk.lines[2], DiffLine::Context(_)));
    }
}
