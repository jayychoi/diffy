//! 핵심 데이터 구조체

use serde::Serialize;

/// 리뷰 상태
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ReviewStatus {
    Pending,
    Accepted,
    Rejected,
}

/// diff 한 줄
#[derive(Clone, Debug, Serialize)]
pub enum DiffLine {
    Context(String),   // ' '로 시작
    Added(String),     // '+'로 시작
    Removed(String),   // '-'로 시작
    NoNewline,         // '\ No newline at end of file'
}

/// 헌크 하나
#[derive(Clone, Debug, Serialize)]
pub struct Hunk {
    pub header: String,           // @@ -a,b +c,d @@ ...
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub lines: Vec<DiffLine>,
    pub status: ReviewStatus,
    pub comment: Option<String>,
}

/// 파일 하나의 diff
#[derive(Clone, Debug, Serialize)]
pub struct FileDiff {
    pub old_path: String,         // 표시용 경로 (접두사 없음): "src/main.rs"
    pub new_path: String,         // 표시용 경로 (접두사 없음): "src/main.rs"
    pub raw_old_path: String,     // 출력용 원본 경로: "a/src/main.rs"
    pub raw_new_path: String,     // 출력용 원본 경로: "b/src/main.rs"
    pub hunks: Vec<Hunk>,
    pub is_binary: bool,          // 바이너리 파일 여부
}

impl FileDiff {
    pub fn lines_added(&self) -> usize {
        self.hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| matches!(l, DiffLine::Added(_)))
            .count()
    }

    pub fn lines_removed(&self) -> usize {
        self.hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| matches!(l, DiffLine::Removed(_)))
            .count()
    }

    pub fn review_summary(&self) -> FileReviewSummary {
        if self.hunks.is_empty() {
            return FileReviewSummary::Empty;
        }
        if self.hunks.iter().all(|h| h.status == ReviewStatus::Accepted) {
            FileReviewSummary::AllAccepted
        } else if self.hunks.iter().any(|h| h.status == ReviewStatus::Rejected) {
            FileReviewSummary::HasRejected
        } else if self.hunks.iter().all(|h| h.status == ReviewStatus::Pending) {
            FileReviewSummary::AllPending
        } else {
            FileReviewSummary::Partial
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileReviewSummary {
    AllPending,
    AllAccepted,
    HasRejected,
    Partial,
    Empty,
}

/// 전체 diff
#[derive(Clone, Debug, Serialize)]
pub struct Diff {
    pub files: Vec<FileDiff>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_hunk(lines: Vec<DiffLine>, status: ReviewStatus) -> Hunk {
        Hunk {
            header: "@@ -1,1 +1,1 @@".to_string(),
            old_start: 1,
            old_count: 1,
            new_start: 1,
            new_count: 1,
            lines,
            status,
            comment: None,
        }
    }

    fn make_file(hunks: Vec<Hunk>) -> FileDiff {
        FileDiff {
            old_path: "test.rs".to_string(),
            new_path: "test.rs".to_string(),
            raw_old_path: "a/test.rs".to_string(),
            raw_new_path: "b/test.rs".to_string(),
            hunks,
            is_binary: false,
        }
    }

    #[test]
    fn test_lines_added_removed() {
        let file = make_file(vec![
            make_hunk(
                vec![
                    DiffLine::Context("ctx".to_string()),
                    DiffLine::Added("new1".to_string()),
                    DiffLine::Added("new2".to_string()),
                    DiffLine::Removed("old1".to_string()),
                ],
                ReviewStatus::Pending,
            ),
            make_hunk(
                vec![
                    DiffLine::Added("new3".to_string()),
                    DiffLine::Removed("old2".to_string()),
                    DiffLine::Removed("old3".to_string()),
                ],
                ReviewStatus::Pending,
            ),
        ]);
        assert_eq!(file.lines_added(), 3);
        assert_eq!(file.lines_removed(), 3);
    }

    #[test]
    fn test_review_summary_all_pending() {
        let file = make_file(vec![
            make_hunk(vec![], ReviewStatus::Pending),
            make_hunk(vec![], ReviewStatus::Pending),
        ]);
        assert_eq!(file.review_summary(), FileReviewSummary::AllPending);
    }

    #[test]
    fn test_review_summary_all_accepted() {
        let file = make_file(vec![
            make_hunk(vec![], ReviewStatus::Accepted),
            make_hunk(vec![], ReviewStatus::Accepted),
        ]);
        assert_eq!(file.review_summary(), FileReviewSummary::AllAccepted);
    }

    #[test]
    fn test_review_summary_has_rejected() {
        let file = make_file(vec![
            make_hunk(vec![], ReviewStatus::Accepted),
            make_hunk(vec![], ReviewStatus::Rejected),
        ]);
        assert_eq!(file.review_summary(), FileReviewSummary::HasRejected);
    }

    #[test]
    fn test_review_summary_partial() {
        let file = make_file(vec![
            make_hunk(vec![], ReviewStatus::Accepted),
            make_hunk(vec![], ReviewStatus::Pending),
        ]);
        assert_eq!(file.review_summary(), FileReviewSummary::Partial);
    }

    #[test]
    fn test_review_summary_empty() {
        let file = make_file(vec![]);
        assert_eq!(file.review_summary(), FileReviewSummary::Empty);
    }
}
