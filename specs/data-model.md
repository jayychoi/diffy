# Data Model — diffy

## 핵심 타입 계층

```
Diff
└── files: Vec<FileDiff>
    ├── old_path: String          "src/main.rs"
    ├── new_path: String          "src/main.rs"
    ├── raw_old_path: String      "a/src/main.rs"
    ├── raw_new_path: String      "b/src/main.rs"
    ├── is_binary: bool
    └── hunks: Vec<Hunk>
        ├── header: String        "@@ -10,5 +10,6 @@ fn main()"
        ├── old_start: u32        10
        ├── old_count: u32        5
        ├── new_start: u32        10
        ├── new_count: u32        6
        ├── status: ReviewStatus  Pending | Accepted | Rejected
        ├── comment: Option<String>
        └── lines: Vec<DiffLine>
            ├── Context(String)   " " 접두사 라인
            ├── Added(String)     "+" 접두사 라인
            ├── Removed(String)   "-" 접두사 라인
            └── NoNewline         "\ No newline at end of file"
```

## 열거형

### ReviewStatus

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub enum ReviewStatus {
    Pending,    // 아직 리뷰하지 않음 (기본값)
    Accepted,   // 사용자가 수락
    Rejected,   // 사용자가 거부
}
```

### DiffLine

```rust
#[derive(Clone, Debug, Serialize)]
pub enum DiffLine {
    Context(String),  // 변경 없는 컨텍스트 라인
    Added(String),    // 추가된 라인
    Removed(String),  // 삭제된 라인
    NoNewline,        // 파일 끝 개행 없음 표시
}
```

### FileReviewSummary

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileReviewSummary {
    AllPending,   // 모든 hunk가 Pending
    AllAccepted,  // 모든 hunk가 Accepted
    HasRejected,  // 하나 이상 Rejected
    Partial,      // Accepted + Pending 혼합
    Empty,        // hunk 없음
}
```

파일 트리 아이콘 매핑: `·` AllPending, `✓` AllAccepted, `✗` HasRejected, `~` Partial

## TUI 상태

### AppMode (상태 머신)

```rust
pub(super) enum AppMode {
    Normal,       // 기본 탐색/리뷰 모드
    Help,         // 도움말 오버레이
    ConfirmQuit,  // 종료 확인
    PendingG,     // 'g' 입력 후 대기 (gg로 첫 hunk 이동)
    Search,       // 검색어 입력 중
    Stats,        // 통계 오버레이
    CommentEdit,  // 코멘트 입력 중
}
```

### DiffViewMode

```rust
pub(super) enum DiffViewMode {
    Unified,     // 통합 뷰 (기본)
    SideBySide,  // 양쪽 비교 뷰
}
```

### AppState (핵심 필드)

```rust
pub(super) struct AppState {
    // 데이터
    pub diff: Diff,

    // 내비게이션
    pub file_index: usize,
    pub hunk_index: usize,

    // UI 모드
    pub mode: AppMode,
    pub should_quit: bool,
    pub diff_view_mode: DiffViewMode,

    // 뷰포트
    pub viewport_offset: usize,
    pub viewport_height: usize,

    // 기능 토글
    pub show_file_tree: bool,
    pub show_mouse: bool,
    pub show_highlight: bool,

    // 언두
    pub undo_stack: Vec<UndoEntry>,

    // 검색
    pub search_query: String,
    pub search_matches: Vec<SearchMatch>,
    pub search_index: Option<usize>,

    // 통계/코멘트
    pub stats_cursor: usize,
    pub comment_input: String,
}
```

### UndoEntry

```rust
pub(super) struct UndoEntry {
    pub file_index: usize,
    pub hunk_index: usize,
    pub old_status: ReviewStatus,
    pub old_comment: Option<String>,
}
```

### SearchMatch

```rust
pub(super) struct SearchMatch {
    pub file_index: usize,
    pub hunk_index: usize,
    pub line_index: usize,
}
```

## Action 열거형 (45가지 액션)

```rust
pub(super) enum Action {
    // 내비게이션
    NextHunk, PrevHunk, NextFile, PrevFile,
    FirstHunk, LastHunk, NextPending,
    ScrollUp, ScrollDown,

    // 리뷰
    Accept, Reject, Toggle,
    AcceptAll, RejectAll,
    Undo,

    // 모드 전환
    EnterPendingG, EnterSearch, EnterComment,
    ToggleHelp, ToggleStats, ConfirmQuit,
    CancelPendingG,

    // 토글
    ToggleFileTree, ToggleSideBySide,
    ToggleHighlight, ToggleMouse,

    // 검색
    SearchChar(char), SearchBackspace, SearchSubmit,
    NextMatch, PrevMatch,

    // 코멘트
    CommentChar(char), CommentBackspace, CommentSubmit,

    // 통계
    StatsCursorUp, StatsCursorDown, StatsNavigate,

    // 기타
    Quit, CancelQuit, CloseOverlay,
    Noop,
}
```

## 설정 모델

### Config

```rust
pub struct Config {
    pub defaults: Defaults,
}

pub struct Defaults {
    pub highlight: bool,    // 기본: false
    pub mouse: bool,        // 기본: false
    pub view: ViewMode,     // 기본: Unified
    pub file_tree: bool,    // 기본: true
}

pub enum ViewMode {
    Unified,
    SideBySide,
}
```

### config.toml 예시

```toml
[defaults]
highlight = true
mouse = true
view = "side-by-side"
file_tree = false
```

## JSON 출력 스키마

```json
{
  "files": [
    {
      "old_path": "string",
      "new_path": "string",
      "raw_old_path": "string",
      "raw_new_path": "string",
      "is_binary": false,
      "hunks": [
        {
          "header": "string",
          "old_start": 10,
          "old_count": 5,
          "new_start": 10,
          "new_count": 6,
          "status": "accepted | rejected | pending",
          "comment": "string | null",
          "lines": [
            { "Context": "string" },
            { "Added": "string" },
            { "Removed": "string" },
            "NoNewline"
          ]
        }
      ]
    }
  ]
}
```

## 데이터 흐름

```
입력 (git diff 텍스트)
  │
  ▼
parse::parse_diff()
  │ 텍스트 → Diff { files: Vec<FileDiff> }
  │ 모든 hunk의 status = Pending
  ▼
tui::run(diff, config)
  │ 사용자 인터랙션으로 status 변경
  │ comment 추가
  │ undo_stack에 이전 상태 저장
  ▼
reviewed Diff
  │
  ├─→ output::write_diff()      // accepted hunk만 unified diff로 출력
  ├─→ output::write_json()      // 전체 리뷰 결과 JSON
  ├─→ hook::write_feedback()    // rejected hunk 피드백 (stderr)
  └─→ revert::generate_reverse_patch() // rejected hunk 역패치
```
