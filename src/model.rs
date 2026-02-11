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

/// 전체 diff
#[derive(Clone, Debug, Serialize)]
pub struct Diff {
    pub files: Vec<FileDiff>,
}
