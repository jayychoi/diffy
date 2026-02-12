# TUI Design — diffy

## 레이아웃 구조

```
┌──────────────────────────────────────────────────────┐
│ File Bar (1줄)                                        │
│  src/main.rs  +15 -3  [2/5 files]                    │
├───────────┬──────────────────────────────────────────┤
│ File Tree │ Diff View                                │
│ (30 cols) │                                          │
│           │  ▸ @@ -10,5 +10,6 @@ fn main() [✓]      │
│ ✓ main.rs │                                          │
│ ~ lib.rs  │  ▾ @@ -25,3 +25,8 @@ fn run() [ ]       │
│ ✗ test.rs │   25│  use std::io;                      │
│ · cfg.rs  │   26│- old_function();                   │
│           │   27│+ new_function();                   │
│           │   28│  Ok(())                            │
│           │                                          │
│           │  💬 이 함수는 유지해주세요                  │
│           │                                          │
│           │  ▸ @@ -40,2 +45,3 @@ fn helper() [ ]     │
├───────────┴──────────────────────────────────────────┤
│ Status Bar (1줄)                                      │
│  [3/10 reviewed] a:accept r:reject u:undo q:quit     │
└──────────────────────────────────────────────────────┘
```

## 렌더링 영역

### File Bar (상단 1줄)

- 현재 파일 경로
- 추가/삭제 라인 수 (`+N -M`)
- 파일 인덱스 (`[X/Y files]`)

### File Tree (좌측 사이드바, 30컬럼)

- `f` 키로 토글 가능
- 파일별 리뷰 상태 아이콘:
  - `✓` 모든 hunk accepted
  - `✗` rejected hunk 있음
  - `~` 부분 리뷰됨
  - `·` 미리뷰 (pending)
- 긴 경로는 잘림 처리
- 현재 파일 하이라이트 (반전 스타일)

### Diff View (메인 영역)

#### Unified 뷰 (기본)

- **Virtual Document 모델**: 전체 문서를 구성한 뒤 viewport로 슬라이싱
- 현재 hunk만 확장 (라인 표시), 나머지는 헤더 1줄로 축소
- 라인 번호 표시 (old/new 분리)
- 색상 코딩:
  - 초록: 추가된 라인
  - 빨강: 삭제된 라인
  - 회색: 컨텍스트 라인
  - 노랑: 검색 매치
  - 밝은 노랑: 현재 검색 매치
- Hunk 헤더에 리뷰 상태 표시: `[ ]`, `[✓]`, `[✗]`
- 코멘트가 있으면 hunk 아래에 `💬 내용` 표시

#### Side-by-Side 뷰

- `d` 키로 토글
- 터미널 폭 ≥ 100 필요 (미만 시 자동 unified 폴백)
- 좌: 원본 (삭제된 라인), 우: 수정본 (추가된 라인)
- 삭제/추가 라인 페어링 알고리즘:
  1. 연속 삭제 라인 버퍼링
  2. 연속 추가 라인 버퍼링
  3. Context 라인에서 버퍼 플러시 (max length로 페어링)

### Status Bar (하단 1줄)

모드별 다른 정보 표시:

| 모드 | 표시 내용 |
|------|----------|
| Normal | `[N/M reviewed] a:accept r:reject u:undo q:quit` |
| Search | `search: 검색어▎` |
| CommentEdit | `comment: 코멘트 내용▎` |
| ConfirmQuit | `Quit? (y/n) N pending hunks` |
| PendingG | `g pressed — press g again for first hunk` |

## 오버레이

### Help 오버레이 (`?` 키)

중앙 모달로 전체 키바인딩 표시. 아무 키나 누르면 닫힘.

### Stats 오버레이 (`s` 키)

파일별 리뷰 진행률 표시:
- 파일명, hunk 수, accepted/rejected/pending 카운트
- `j`/`k`로 커서 이동
- `Enter`로 해당 파일로 이동
- `s`/`Esc`/`q`로 닫기

## 키바인딩 전체 맵

### 내비게이션

| 키 | 동작 | 컨텍스트 |
|----|------|----------|
| `j` / `↓` | 다음 hunk | Normal |
| `k` / `↑` | 이전 hunk | Normal |
| `n` | 다음 파일 (검색 시: 다음 매치) | Normal |
| `N` | 이전 파일 (검색 시: 이전 매치) | Normal |
| `Ctrl+d` / `PgDn` | 반 페이지 아래 | Normal |
| `Ctrl+u` / `PgUp` | 반 페이지 위 | Normal |
| `g` → `g` | 첫 hunk (vim-style) | Normal → PendingG |
| `G` | 마지막 hunk | Normal |
| `Tab` | 다음 pending hunk | Normal |

### 리뷰

| 키 | 동작 |
|----|------|
| `a` | 현재 hunk accept |
| `r` | 현재 hunk reject |
| `Space` / `Enter` | 현재 hunk 상태 토글 |
| `A` | 모든 hunk accept |
| `R` | 모든 hunk reject |
| `u` | 마지막 리뷰 결정 언두 |

### 코멘트

| 키 | 동작 | 컨텍스트 |
|----|------|----------|
| `c` | 코멘트 편집 시작 | Normal |
| `Enter` | 코멘트 저장 | CommentEdit |
| `Esc` | 코멘트 취소 | CommentEdit |
| `Backspace` | 문자 삭제 | CommentEdit |
| 문자 입력 | 코멘트에 추가 | CommentEdit |

### 검색

| 키 | 동작 | 컨텍스트 |
|----|------|----------|
| `/` | 검색 모드 진입 | Normal |
| `Enter` | 검색 실행 | Search |
| `Esc` | 검색 취소 | Search |
| `n` | 다음 매치 | Normal (검색 활성 시) |
| `N` | 이전 매치 | Normal (검색 활성 시) |

### 뷰/토글

| 키 | 동작 |
|----|------|
| `f` | 파일 트리 토글 |
| `d` | 통합/양쪽 비교 뷰 토글 |
| `h` | 구문 강조 토글 |
| `m` | 마우스 지원 토글 |
| `s` | 통계 오버레이 토글 |
| `?` | 도움말 오버레이 토글 |

### 종료

| 키 | 동작 | 컨텍스트 |
|----|------|----------|
| `q` / `Esc` | 종료 확인 프롬프트 | Normal |
| `y` / `Enter` | 종료 확인 | ConfirmQuit |
| `n` / `Esc` | 종료 취소 | ConfirmQuit |

## 구문 강조

키워드 기반 하이라이팅 (파일 확장자로 언어 감지):

| 언어 | 확장자 | 키워드 수 |
|------|--------|----------|
| Rust | .rs | 42 |
| TypeScript/JavaScript | .ts, .js, .tsx, .jsx | 27 |
| Python | .py | 30 |
| Go | .go | 24 |
| Java | .java | 22 |
| C/C++ | .c, .cpp, .h, .hpp | 33 |
| Ruby | .rb | 20 |

하이라이팅 규칙:
1. `//` 또는 `#` → 주석 (DarkGray)
2. `"..."` 또는 `'...'` → 문자열 (Yellow) — 이스케이프 처리
3. 키워드 → Magenta
4. 기타 → 기본 스타일 (추가=Green, 삭제=Red, 컨텍스트=DarkGray)

## 마우스 지원

`m` 키로 토글 (`config.toml`에서 기본값 설정 가능).

지원 이벤트:
- 파일 트리 클릭 → 해당 파일로 이동
- 스크롤 업/다운 → 뷰포트 스크롤

## 뷰포트 스크롤링

- Virtual document height 계산: 모든 hunk의 높이 합산 (현재 hunk는 전체 라인, 나머지는 1줄)
- `ensure_visible()`: 현재 hunk가 viewport 안에 있도록 offset 자동 조정
- `Ctrl+u`/`Ctrl+d`: viewport_height / 2 만큼 스크롤
- 마우스 스크롤: 3줄씩 이동
