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
            hunks.extend(split_hunk_on_context(&hunk));
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

/// 헌크를 변경 그룹별로 분할
///
/// 하나의 헌크에 여러 변경 그룹(Added/Removed 라인)이 있고 그 사이에
/// context 라인만 존재하는 경우, 각 변경 그룹을 별도 헌크로 분할한다.
/// 각 sub-hunk는 최대 3줄의 앞뒤 context를 포함한다.
fn split_hunk_on_context(hunk: &Hunk) -> Vec<Hunk> {
    // 변경 그룹 식별: (start_idx, end_idx) 튜플의 벡터
    let mut groups: Vec<(usize, usize)> = Vec::new();
    let mut group_start: Option<usize> = None;

    for (i, line) in hunk.lines.iter().enumerate() {
        match line {
            DiffLine::Added(_) | DiffLine::Removed(_) => {
                if group_start.is_none() {
                    group_start = Some(i);
                }
            }
            DiffLine::NoNewline => {
                // NoNewline은 앞의 Added/Removed 라인에 붙어있음 - 무시
            }
            DiffLine::Context(_) => {
                if let Some(start) = group_start {
                    groups.push((start, i - 1));
                    group_start = None;
                }
            }
        }
    }

    // 마지막 그룹이 열려있으면 닫기
    if let Some(start) = group_start {
        groups.push((start, hunk.lines.len() - 1));
    }

    // 변경 그룹이 1개 이하면 분할하지 않음
    if groups.len() <= 1 {
        return vec![hunk.clone()];
    }

    // 각 그룹을 별도 헌크로 분할
    let mut result = Vec::new();
    const CONTEXT_SIZE: usize = 3;

    for (group_start, group_end) in groups {
        // 앞 context 수집 (최대 3줄)
        let context_before_start = group_start.saturating_sub(CONTEXT_SIZE);
        let mut context_before = Vec::new();
        for i in context_before_start..group_start {
            if matches!(hunk.lines[i], DiffLine::Context(_)) {
                context_before.push(hunk.lines[i].clone());
            }
        }

        // 변경 라인 수집 (NoNewline 포함)
        let mut change_lines = Vec::new();
        for i in group_start..=group_end {
            change_lines.push(hunk.lines[i].clone());
        }
        // 변경 그룹 바로 뒤의 NoNewline도 포함
        if group_end + 1 < hunk.lines.len()
            && matches!(hunk.lines[group_end + 1], DiffLine::NoNewline)
        {
            change_lines.push(hunk.lines[group_end + 1].clone());
        }

        // 뒤 context 수집 (최대 3줄)
        let context_after_start = group_end + 1;
        let context_after_start = if context_after_start < hunk.lines.len()
            && matches!(hunk.lines[context_after_start], DiffLine::NoNewline)
        {
            context_after_start + 1
        } else {
            context_after_start
        };
        let context_after_end = (context_after_start + CONTEXT_SIZE).min(hunk.lines.len());
        let mut context_after = Vec::new();
        for i in context_after_start..context_after_end {
            if matches!(hunk.lines[i], DiffLine::Context(_)) {
                context_after.push(hunk.lines[i].clone());
            }
        }

        // 새 헌크의 lines 구성
        let mut new_lines = Vec::new();
        new_lines.extend(context_before.clone());
        new_lines.extend(change_lines.clone());
        new_lines.extend(context_after.clone());

        // 라인 번호 계산
        // context_before_start부터 시작해서 old/new 라인 번호를 추적
        let mut old_line = hunk.old_start;
        let mut new_line = hunk.new_start;

        // context_before_start까지 진행
        for i in 0..context_before_start {
            match &hunk.lines[i] {
                DiffLine::Context(_) => {
                    old_line += 1;
                    new_line += 1;
                }
                DiffLine::Added(_) => {
                    new_line += 1;
                }
                DiffLine::Removed(_) => {
                    old_line += 1;
                }
                DiffLine::NoNewline => {}
            }
        }

        let new_old_start = old_line;
        let new_new_start = new_line;

        // new_lines를 순회하며 old_count, new_count 계산
        let mut new_old_count = 0;
        let mut new_new_count = 0;

        for line in &new_lines {
            match line {
                DiffLine::Context(_) => {
                    new_old_count += 1;
                    new_new_count += 1;
                }
                DiffLine::Added(_) => {
                    new_new_count += 1;
                }
                DiffLine::Removed(_) => {
                    new_old_count += 1;
                }
                DiffLine::NoNewline => {}
            }
        }

        // 새 헤더 생성
        let new_header = format!(
            "@@ -{},{} +{},{} @@",
            new_old_start, new_old_count, new_new_start, new_new_count
        );

        result.push(Hunk {
            header: new_header,
            old_start: new_old_start,
            old_count: new_old_count,
            new_start: new_new_start,
            new_count: new_new_count,
            lines: new_lines,
            status: hunk.status,
            comment: hunk.comment.clone(),
        });
    }

    result
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

    #[test]
    fn test_split_hunk_single_group() {
        let input = indoc! {"
            --- a/file.txt
            +++ b/file.txt
            @@ -1,5 +1,4 @@
             line1
             line2
            -deleted line
             line3
             line4
        "};

        let diff = parse_diff(input).unwrap();
        assert_eq!(diff.files.len(), 1);

        let file = &diff.files[0];
        // 단일 변경 그룹이므로 분할되지 않음
        assert_eq!(file.hunks.len(), 1);

        let hunk = &file.hunks[0];
        assert_eq!(hunk.lines.len(), 5);
    }

    #[test]
    fn test_split_hunk_two_groups() {
        let input = indoc! {"
            --- a/file.txt
            +++ b/file.txt
            @@ -20,15 +20,13 @@
             context line 20
             context line 21
             context line 22
             context line 23
             context line 24
            -deleted line 25
             context line 26
             context line 27
             context line 28
             context line 29
             context line 30
            -deleted line 31
             context line 32
             context line 33
             context line 34
        "};

        let diff = parse_diff(input).unwrap();
        assert_eq!(diff.files.len(), 1);

        let file = &diff.files[0];
        // 2개의 변경 그룹으로 분할되어야 함
        assert_eq!(file.hunks.len(), 2);

        // 첫 번째 헌크: line 25 삭제
        let hunk1 = &file.hunks[0];
        assert_eq!(hunk1.old_start, 22);
        assert_eq!(hunk1.new_start, 22);
        // 3 context before + 1 removed + 3 context after
        assert_eq!(hunk1.lines.len(), 7);
        assert!(matches!(hunk1.lines[3], DiffLine::Removed(_)));

        // 두 번째 헌크: line 31 삭제
        let hunk2 = &file.hunks[1];
        assert_eq!(hunk2.old_start, 28);
        assert_eq!(hunk2.new_start, 27);
        // 3 context before + 1 removed + 3 context after
        assert_eq!(hunk2.lines.len(), 7);
        assert!(matches!(hunk2.lines[3], DiffLine::Removed(_)));
    }

    #[test]
    fn test_split_hunk_three_groups() {
        let input = indoc! {"
            --- a/file.txt
            +++ b/file.txt
            @@ -1,20 +1,17 @@
             line1
             line2
            -deleted1
             line3
             line4
             line5
             line6
            +added1
             line7
             line8
             line9
             line10
            -deleted2
             line11
             line12
             line13
        "};

        let diff = parse_diff(input).unwrap();
        assert_eq!(diff.files.len(), 1);

        let file = &diff.files[0];
        // 3개의 변경 그룹으로 분할되어야 함
        assert_eq!(file.hunks.len(), 3);

        // 첫 번째 헌크: deleted1
        let hunk1 = &file.hunks[0];
        assert!(matches!(hunk1.lines.iter().find(|l| matches!(l, DiffLine::Removed(_))), Some(_)));

        // 두 번째 헌크: added1
        let hunk2 = &file.hunks[1];
        assert!(matches!(hunk2.lines.iter().find(|l| matches!(l, DiffLine::Added(_))), Some(_)));

        // 세 번째 헌크: deleted2
        let hunk3 = &file.hunks[2];
        assert!(matches!(hunk3.lines.iter().find(|l| matches!(l, DiffLine::Removed(_))), Some(_)));
    }

    #[test]
    fn test_split_hunk_no_newline() {
        let input = indoc! {"
            --- a/file.txt
            +++ b/file.txt
            @@ -1,10 +1,9 @@
             line1
             line2
            -deleted1
            \\ No newline at end of file
             line3
             line4
             line5
             line6
            -deleted2
             line7
        "};

        let diff = parse_diff(input).unwrap();
        assert_eq!(diff.files.len(), 1);

        let file = &diff.files[0];
        // 2개의 변경 그룹으로 분할
        assert_eq!(file.hunks.len(), 2);

        // 첫 번째 헌크: deleted1 + NoNewline
        let hunk1 = &file.hunks[0];
        // NoNewline이 변경 그룹에 포함되어야 함
        assert!(hunk1.lines.iter().any(|l| matches!(l, DiffLine::NoNewline)));
        assert!(hunk1.lines.iter().any(|l| matches!(l, DiffLine::Removed(_))));

        // 두 번째 헌크: deleted2
        let hunk2 = &file.hunks[1];
        assert!(hunk2.lines.iter().any(|l| matches!(l, DiffLine::Removed(_))));
    }

    #[test]
    fn test_parse_diff_splits_hunks() {
        // 통합 테스트: parse_diff가 split_hunk_on_context를 올바르게 호출하는지 확인
        let input = indoc! {"
            diff --git a/test.txt b/test.txt
            --- a/test.txt
            +++ b/test.txt
            @@ -10,12 +10,10 @@
             context10
             context11
            +added12
             context13
             context14
             context15
             context16
            -removed17
             context18
             context19
        "};

        let diff = parse_diff(input).unwrap();
        assert_eq!(diff.files.len(), 1);

        let file = &diff.files[0];
        // 2개의 헌크로 분할되어야 함
        assert_eq!(file.hunks.len(), 2);

        // 각 헌크가 올바른 라인 번호를 가지는지 확인
        assert!(file.hunks[0].old_start >= 10);
        assert!(file.hunks[1].old_start >= 10);

        // 각 헌크가 변경 사항을 포함하는지 확인
        assert!(file.hunks[0].lines.iter().any(|l| matches!(l, DiffLine::Added(_))));
        assert!(file.hunks[1].lines.iter().any(|l| matches!(l, DiffLine::Removed(_))));
    }
}
