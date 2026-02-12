# CLAUDE.md — diffy 프로젝트 가이드

## 프로젝트 개요

**diffy**는 Claude Code용 인터랙티브 TUI diff 리뷰어입니다. 터미널에서 AI가 생성한 코드 변경사항을 hunk 단위로 accept/reject하고, 구조화된 피드백을 Claude Code에 자동 전달합니다.

- **크레이트명**: `diffy-tui` (바이너리: `diffy`)
- **버전**: 0.3.0
- **언어**: Rust (edition 2024, MSRV 1.85.0)
- **라이선스**: MIT
- **저장소**: https://github.com/jaykang-heo/diffy

## 빌드 & 테스트

```bash
cargo build                    # 빌드
cargo build --release          # 릴리스 빌드
cargo test                     # 전체 테스트 실행
cargo clippy -- -D warnings    # 린트 (CI와 동일)
cargo fmt --check              # 포맷 검사
cargo fmt                      # 자동 포맷
```

## 프로젝트 구조

```
src/
├── main.rs          # 엔트리포인트, 모드 라우팅 (pipe/CLI/restore)
├── lib.rs           # 라이브러리 모듈 re-export
├── cli.rs           # clap 기반 CLI 인자 파싱
├── config.rs        # 설정 파일 로드 (~/.config/diffy/config.toml)
├── model.rs         # 핵심 데이터 구조체 (Diff, FileDiff, Hunk, ReviewStatus)
├── parse.rs         # unified diff 파서
├── git.rs           # git CLI 래퍼 (diff, repo 감지)
├── hook.rs          # Claude Code hook 피드백 생성
├── output.rs        # diff/JSON 출력
├── revert.rs        # git stash 기반 백업/복원/역방향 패치
├── tty.rs           # TTY 감지
└── tui/
    ├── mod.rs       # TUI 메인 루프, /dev/tty, CleanupGuard
    ├── state.rs     # 앱 상태 머신 (내비게이션, 언두, 검색, 뷰포트)
    ├── input.rs     # 키보드 입력 → Action 매핑
    ├── render.rs    # ratatui 기반 UI 렌더링
    └── highlight.rs # 키워드 기반 구문 강조
```

## 아키텍처 핵심

### 실행 모드 (main.rs)

1. **`--restore`**: 백업 복원 후 종료
2. **Pipe 모드**: stdin이 파이프일 때 → `git diff | diffy | git apply`
3. **CLI 모드**: stdin이 TTY일 때 → `diffy --staged/--head/--ref`

### 데이터 흐름

```
git diff → parse::parse_diff() → Diff → tui::run() → reviewed Diff
  → [--apply] revert::generate_reverse_patch() + apply_reverse()
  → [--hook-mode] hook::write_feedback() → stderr (exit 0 or 2)
  → output::write_diff() or write_json() → stdout
```

### TUI 아키텍처

- **터미널 제어**: `/dev/tty`를 직접 열어 TUI 렌더링 (stdin/stdout과 분리)
- **CleanupGuard**: RAII 패턴으로 패닉 시에도 터미널 상태 복원
- **상태 머신**: `AppMode` enum — Normal, Help, ConfirmQuit, PendingG, Search, Stats, CommentEdit
- **이벤트 루프**: crossterm 이벤트 → `handle_key()` → `Action` → `apply_action()` → 상태 변경 → 렌더링

### 종료 코드

- `0` — 모든 hunk accepted (또는 출력 있음)
- `1` — 에러
- `2` — reject된 hunk 있음 (hook-mode에서 Claude에 피드백 전달)

## 코딩 컨벤션

### Rust 스타일

- `cargo fmt` 적용 (표준 rustfmt)
- `cargo clippy -- -D warnings` 통과 필수
- `anyhow::Result` 사용 (에러 핸들링)
- 모듈 내부 타입은 `pub(super)` 가시성 사용
- 한국어 주석 사용 (코드 내 독스트링, 주석)

### 테스트

- 각 모듈 하단에 `#[cfg(test)] mod tests` 블록
- 테스트 헬퍼: `make_hunk()`, `make_file()` 등 픽스처 팩토리
- `indoc!` 매크로로 멀티라인 diff 픽스처 작성
- `pretty_assertions`으로 실패 시 가독성 향상

### 파일 구성 원칙

- 관심사별 모듈 분리 (파싱, 깃, TUI, 출력 등)
- TUI 관련 코드는 `tui/` 서브모듈에 격리
- 모든 데이터 구조체는 `model.rs`에 집중
- 외부 프로세스 호출은 `git.rs`에 캡슐화

## 주요 의존성

| 크레이트 | 용도 |
|----------|------|
| `ratatui` + `crossterm` | TUI 프레임워크 + 터미널 제어 |
| `clap` (derive) | CLI 인자 파싱 |
| `serde` + `serde_json` | JSON 직렬화 |
| `toml` | 설정 파일 파싱 |
| `anyhow` | 에러 핸들링 |

## Claude Code 연동

### Stop Hook 동작 방식

1. Claude Code가 파일 수정 완료 → Stop 이벤트
2. Stop hook이 `diffy --hook-mode --apply` 실행
3. 사용자가 TUI에서 hunk별 accept/reject
4. reject된 hunk 자동 revert (`--apply`)
5. exit code 2 + stderr 피드백 → Claude가 읽고 재작업

### 피드백 포맷 (hook.rs)

```
[diffy review result]
rejected N of M hunks.

- path/to/file.rs (lines X-Y): rejected
  comment: 사용자 코멘트
  ```diff
  -삭제된 라인
  +추가된 라인
  ```

please fix the rejected hunks and try again.
```

- 최대 크기: 10KB (환경변수 `DIFFY_FEEDBACK_MAX_SIZE`로 조정 가능)

## CI/CD

- **ci.yml**: push/PR → test (ubuntu + macos), clippy, fmt
- **release.yml**: `v*` 태그 → 4 플랫폼 크로스 컴파일 → GitHub Release
  - x86_64-unknown-linux-gnu
  - aarch64-unknown-linux-gnu (cross)
  - x86_64-apple-darwin
  - aarch64-apple-darwin

## 환경 변수

| 변수 | 설명 | 기본값 |
|------|------|--------|
| `DIFFY_FEEDBACK_MAX_SIZE` | hook 피드백 최대 바이트 | 10240 |
| `XDG_CONFIG_HOME` | 설정 디렉터리 경로 | `~/.config` |

## 생성 파일

| 경로 | 용도 |
|------|------|
| `~/.config/diffy/config.toml` | 사용자 설정 파일 |
| `.diffy/backup-refs` | git stash SHA 목록 (최대 10개) |
