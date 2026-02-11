# diffy — Claude Code용 TUI Diff Reviewer

## 프로젝트 개요

Claude Code 사용자들은 터미널에서 코드 작업을 하면서 diff를 확인하기 위해 IDE를 열어야 하는 불편함이 있다. IDE는 램과 리소스를 많이 차지하지만, 실제로는 diff를 보기 위한 용도로만 사용되는 경우가 많다. `diffy`는 이 문제를 해결하는 **터미널 기반 TUI diff 리뷰 도구**로, Claude Code 플러그인으로 통합되어 AI가 수정한 코드를 hunk 단위로 accept/reject하고 그 피드백을 Claude Code에 자동 전달하는 것이 핵심이다.

---

## 기존 도구 분석 및 차별점

### 기존 TUI diff 도구들

| 도구 | 특징 | 한계 |
|------|------|------|
| delta | git diff pager, syntax highlighting, side-by-side (★ 20k+) | 읽기 전용, 인터랙션 없음 |
| difftastic | AST 기반 구조 diff | 읽기 전용 |
| diff-so-fancy | git diff 출력 개선 | delta보다 단순 |
| icdiff | side-by-side 컬러 diff | 읽기 전용 |
| lazygit | git 전체 TUI | diff 전용이 아님 |
| tig | git TUI (diff, log, blame) | diff 리뷰에 특화되지 않음 |
| gitui | Rust 기반 git TUI | diff 리뷰에 특화되지 않음 |

### diffy의 차별점

기존 도구들은 **git diff pager**(출력을 예쁘게 보여주기)이거나 **git 전체 워크플로우 TUI**이다. diffy는:

1. **AI 에이전트가 수정한 코드를 리뷰하는 데 특화된 TUI**
2. **Hunk 단위 accept/reject 인터랙션** — IDE의 "Accept Change" 버튼에 해당
3. **Claude Code 플러그인 연동** — 리뷰 결과가 Claude에게 자동 피드백
4. **여러 파일의 변경사항을 파일 트리 + diff로 한눈에** 확인

---

## 기술 스택

**Rust + Ratatui** 선정 이유:
- 싱글 바이너리 배포, 최고 성능, 메모리 안전성
- delta, gitui 등 검증된 TUI 도구들과 동일한 스택
- TUI 생태계가 가장 활발

### 핵심 크레이트

| 용도 | 크레이트 | 비고 |
|------|----------|------|
| TUI 프레임워크 | `ratatui` + `crossterm` | |
| Diff 엔진 | `git2` (libgit2 바인딩) | 기본 diff + hunk 추출 |
| Diff Fallback | `similar` | non-git 파일 비교, git2 hunk 보정용 |
| Syntax highlighting | `syntect` | v0.2에서 추가 |
| CLI 인자 파싱 | `clap` | |
| 에러 핸들링 | `anyhow` + `thiserror` | |
| JSON 직렬화 | `serde` + `serde_json` | 리뷰 결과 출력 |

> **git2 vs similar 역할 분리:** git2가 diff 추출과 hunk 파싱의 1차 엔진이다. similar는 (1) git 저장소가 아닌 환경에서의 fallback, (2) git2가 제공하지 않는 word-level diff 보정이 필요한 경우에만 사용한다.

---

## 기능 로드맵

### Phase 0 — 터미널 제어권 PoC

> **이 단계가 전체 프로젝트의 가능 여부를 결정한다.**

Claude Code의 Stop hook에서 TUI 프로세스를 실행할 때, 터미널 stdin/stdout 제어권을 정상적으로 넘겨받을 수 있는지 검증한다.

**검증 항목:**
- Stop hook에서 TUI 프로세스를 실행하고 stdin/stdout을 점유할 수 있는가?
- TUI 종료 후 Claude Code가 정상적으로 터미널을 되찾는가?
- exit code 2 + stderr 피드백이 Claude Code에 정상 전달되는가?

**PoC 구현:**
```rust
// 최소 PoC: crossterm raw mode 진입 → 간단한 UI 렌더링 → 키 입력 → 종료
fn main() {
    // 1. /dev/tty를 직접 열어서 TUI 렌더링 (hook의 stdin/stdout과 분리)
    // 2. 사용자 입력 받기
    // 3. 종료 시 stderr로 결과 출력
    // 4. exit code 반환
}
```

**대안 전략 (PoC 실패 시):**
- **Plan B:** `/dev/tty`를 직접 열어서 TUI를 렌더링하고, hook의 stdin/stdout은 Claude Code와의 통신 전용으로 사용
- **Plan C:** TUI 대신 non-interactive 모드 — diff를 파일로 저장하고 별도 명령으로 리뷰
- **Plan D:** MCP 서버 방식으로 전환 — Claude Code가 MCP를 통해 diffy를 호출

**완료 기준:** Stop hook에서 TUI가 정상 동작하고, 종료 후 exit code 2 + stderr 메시지가 Claude Code에 전달되는 것을 확인

---

### Phase 1 (v0.1) — 최소 TUI + Hunk 리뷰 + Claude Code 연동

> **핵심 목표:** "Claude Code가 수정한 코드를 hunk 단위로 리뷰하고, reject 피드백이 Claude에게 자동 전달되는" 전체 루프가 동작하는 것.

**diff 표시:**
- Unified diff 뷰 (컬러링: 추가=초록, 삭제=빨강, 컨텍스트=회색)
- 라인 넘버 표시
- Hunk 단위 구분, hunk 간 점프 (`n`/`N` 키)
- 변경된 파일 목록 (좌측 사이드바, 단순 리스트)
- 파일별 변경 통계 (`+3 -1`)

**인터랙션:**
- 파일 목록에서 파일 선택 (`j`/`k`/`Enter`)
- Hunk별 accept (`a`) / reject (`r`) / skip (`s`)
- 전체 accept (`A`) / 전체 reject (`R`)
- 리뷰 완료 시 요약 표시: "5 hunks accepted, 2 rejected"

**Git 연동:**
- `git diff` (unstaged), `git diff --staged`, `git diff HEAD~1` 지원
- reject된 hunk는 안전하게 revert (아래 "안전한 Revert 전략" 참고)

**Claude Code 플러그인 연동 (최소):**
- Stop hook에서 diffy 자동 실행
- 모든 hunk가 accepted이면 → exit code 0 (Claude 종료)
- reject된 hunk가 있으면 → exit code 2 + stderr로 피드백 전달
- 피드백 포맷: 구조화된 텍스트 (JSON은 v0.2에서)

**키바인딩:**

| 키 | 동작 | 컨텍스트 |
|----|------|----------|
| `j`/`k` | 위/아래 이동 | 파일 목록, hunk 목록 |
| `Enter` | 파일 선택 / 확인 | 파일 목록 |
| `Esc` | 뒤로가기 | diff 뷰 → 파일 목록 |
| `n`/`N` | 다음/이전 hunk | diff 뷰 |
| `a` | 현재 hunk accept | diff 뷰 |
| `r` | 현재 hunk reject | diff 뷰 |
| `s` | 현재 hunk skip | diff 뷰 |
| `A` | 현재 파일 전체 accept | diff 뷰 |
| `R` | 현재 파일 전체 reject | diff 뷰 |
| `Tab` | 파일 목록 ↔ diff 뷰 전환 | 전역 |
| `q` | 종료 (리뷰 완료) | 전역 |
| `?` | 키바인딩 도움말 | 전역 |

---

### Phase 2 (v0.2) — 사용성 강화

**뷰 개선:**
- Side-by-side 뷰 (좌: 원본, 우: 수정본), unified/side-by-side 토글 (`v` 키)
- Syntax highlighting (`syntect` 기반)
- diff 내 텍스트 검색 (`/` 키)
- 파일 필터링 (확장자별, 경로별)

**리뷰 기능 확장:**
- reject 시 사유 입력 모달 (선택적, Claude에게 전달될 피드백)
- 인라인 코멘트 달기
- hunk 부분 선택 (hunk 내 특정 라인만 accept/reject)

**출력 강화:**
- 리뷰 결과 JSON 출력 (아래 "리뷰 결과 출력 포맷" 참고)
- 리뷰 히스토리 저장 (`.diffy/` 디렉터리)

**Git 확장:**
- 커밋 간 비교, 브랜치 비교
- `--staged`, `--cached` 등 다양한 diff 범위 지원

---

### Phase 3 (v0.3) — 플러그인 고도화 + 배포

**플러그인 확장:**
- PostToolUse hook으로 Claude의 Edit/Write 시점에 변경 감지/기록
- watch 모드 (파일 변경 실시간 감지, 자동 diff 갱신)
- SKILL.md로 Claude에게 리뷰 결과 해석 방법 안내

**배포:**
- `cargo install diffy` (crates.io)
- GitHub Releases 바이너리 (cross-compile: linux-x64, linux-arm64, macos-x64, macos-arm64)
- Homebrew formula
- CI/CD: GitHub Actions (테스트 + 빌드 + 릴리스)

---

## 안전한 Revert 전략

reject된 hunk를 실제 파일에서 되돌리는 것은 위험할 수 있다. 다음 안전 장치를 적용한다:

1. **자동 백업:** revert 실행 전에 `git stash create`로 현재 상태의 스냅샷을 생성하고, ref를 `.diffy/backup-refs`에 기록
2. **Patch 기반 revert:** `git checkout -p` 대신, reject된 hunk만 역방향 patch로 생성하여 `git apply --reverse`로 적용. 더 정밀하고 예측 가능함
3. **Dry-run 기본:** `--apply` 플래그 없이 실행 시 리뷰 결과만 출력하고 파일을 수정하지 않음. 실제 revert는 `diffy --apply` 또는 리뷰 완료 후 확인 프롬프트에서 명시적 동의 시에만 수행
4. **복구 명령:** `diffy --restore` 로 마지막 revert를 취소하고 백업에서 복원

---

## 프로젝트 구조

```
diffy/
├── Cargo.toml
├── src/
│   ├── main.rs                 # 엔트리포인트, CLI 파싱 (clap)
│   ├── app.rs                  # App 상태 머신 (메인 루프)
│   ├── event.rs                # 키보드/마우스 이벤트 핸들링
│   ├── git/
│   │   ├── mod.rs
│   │   ├── diff.rs             # git2로 diff 추출 + hunk 파싱 (1차 엔진)
│   │   └── apply.rs            # reject된 hunk → 역방향 patch 생성 및 적용
│   ├── diff/
│   │   ├── mod.rs
│   │   ├── hunk.rs             # Hunk, ReviewableHunk 데이터 구조체
│   │   ├── review.rs           # 리뷰 상태 관리 (Accept/Reject/Pending/Skip)
│   │   └── fallback.rs         # similar 기반 fallback diff (non-git 환경용)
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── layout.rs           # 전체 레이아웃 (파일목록 | diff뷰)
│   │   ├── file_list.rs        # 좌측 파일 리스트 패널
│   │   ├── diff_view.rs        # Unified diff 렌더링 (v0.2: side-by-side 추가)
│   │   ├── status_bar.rs       # 하단 상태바 (키 힌트, 진행률)
│   │   ├── summary.rs          # 리뷰 완료 요약 화면
│   │   └── help.rs             # 키바인딩 도움말 오버레이
│   └── output/
│       ├── mod.rs
│       └── report.rs           # 리뷰 결과 텍스트/JSON 출력 (stderr 피드백 포함)
├── plugin/
│   ├── .claude-plugin/
│   │   └── plugin.json         # 플러그인 메타데이터
│   ├── hooks/
│   │   └── stop.sh             # Stop hook: diffy 실행 + exit code 2 피드백
│   └── skills/                 # v0.3
│       └── diff-review/
│           └── SKILL.md
└── tests/
    ├── git_diff_test.rs        # git2 diff 추출 테스트
    ├── hunk_review_test.rs     # 리뷰 상태 전이 테스트
    ├── apply_test.rs           # revert 로직 테스트
    └── fixtures/
        └── sample_repos/       # 테스트용 git repo fixtures
```

---

## 아키텍처

### App 상태 머신

```
┌─────────────┐
│  FileSelect  │ ← 파일 목록에서 파일 선택
└──────┬───────┘
       │ Enter
       ▼
┌─────────────┐
│  HunkReview  │ ← hunk별 accept/reject/skip
└──────┬───────┘
       │ 모든 hunk 처리 완료 또는 q
       ▼
┌─────────────┐
│   Summary    │ ← 리뷰 완료 요약 + apply 확인
└──────┬───────┘
       │ Enter (apply) / q (결과만 출력)
       ▼
      종료 → stderr로 피드백 + exit code 반환
```

> v0.2에서 `FeedbackInput` 상태 추가 (reject 사유 입력 모달)

### 데이터 플로우

```
git2 (repo)
  → diff 추출 (FileDelta 목록 + Hunk 목록)
    → Vec<ReviewableHunk> { status: Pending|Accepted|Rejected|Skipped }
      → UI 렌더링
        → 사용자 입력 → 상태 변경
          → Summary 화면
            → apply 선택 시: backup → 역방향 patch → git apply
            → report: stderr 피드백 출력 (Claude Code용)
```

---

## Claude Code 플러그인 연동 설계

### 핵심 메커니즘: Stop Hook + Exit Code 2

Claude Code의 hooks 시스템에서 **Stop hook**이 연계의 핵심이다. Stop hook이 exit code 2를 반환하면, stderr 메시지가 Claude에게 피드백으로 전달되고 Claude가 작업을 계속한다.

### 워크플로우

```
1. 사용자가 Claude Code에 작업 요청
2. Claude Code가 파일 수정 완료 → Stop 이벤트 발생
3. Stop hook이 diffy TUI를 실행 (/dev/tty로 터미널 접근)
4. 사용자가 TUI에서 hunk별로 accept/reject
5. 분기:
   a. 모든 hunk accepted → exit code 0 → Claude 정상 종료
   b. reject된 hunk 있음:
      → reject된 hunk revert (--apply 모드 시)
      → exit code 2 + stderr로 피드백 전달
      → Claude Code가 피드백을 받고 자동으로 재작업
6. 2~5 반복 (사용자가 만족할 때까지)
```

### 터미널 제어권 전환

Stop hook은 Claude Code의 subprocess로 실행되므로, stdin/stdout이 Claude Code와 연결되어 있다. TUI는 별도로 `/dev/tty`를 열어서 터미널과 직접 통신한다:

```
┌─────────────────────────────────────┐
│ Claude Code (stdin/stdout 점유)      │
│                                      │
│  Stop hook 실행                      │
│   └─ diffy 프로세스                  │
│       ├─ stdin/stdout → Claude Code  │ (사용하지 않음)
│       ├─ /dev/tty → 터미널 직접 연결 │ (TUI 렌더링 + 입력)
│       └─ stderr → Claude Code        │ (피드백 전달)
│                                      │
│  diffy 종료 후                       │
│   └─ exit code + stderr 수집         │
│   └─ Claude Code가 터미널 재점유     │
└─────────────────────────────────────┘
```

### Stop Hook 스크립트 (plugin/hooks/stop.sh)

```bash
#!/bin/bash

# git diff가 없으면 (변경사항 없음) 바로 통과
if git diff --quiet HEAD 2>/dev/null; then
    exit 0
fi

# diffy 실행 — /dev/tty는 diffy 내부에서 직접 열어서 처리
diffy --hook-mode 2>/tmp/diffy-feedback.txt
DIFFY_EXIT=$?

if [ $DIFFY_EXIT -eq 2 ]; then
    # reject된 hunk가 있음 — 피드백을 stderr로 전달
    cat /tmp/diffy-feedback.txt >&2
    exit 2
fi

# 모든 hunk accepted 또는 변경 없음
exit 0
```

### 리뷰 결과 출력 포맷

**v0.1 — 구조화된 텍스트 (stderr 피드백용):**

```
[diffy review result]
rejected 2 of 7 hunks.

- src/auth.ts (lines 42-56): rejected — 기존 에러 핸들링 로직 유지 필요
- src/db.ts (lines 12-18): rejected — 이 쿼리는 성능 문제 있음

please fix the rejected hunks and try again.
```

**v0.2 — JSON (파일 출력, 프로그래매틱 사용):**

```json
{
  "version": 1,
  "summary": {
    "total_hunks": 7,
    "accepted": 5,
    "rejected": 2,
    "skipped": 0
  },
  "files": [
    {
      "path": "src/auth.ts",
      "hunks": [
        { "index": 0, "status": "accepted", "lines": "15-28" },
        {
          "index": 1,
          "status": "rejected",
          "lines": "42-56",
          "reason": "기존 에러 핸들링 로직 유지 필요"
        }
      ]
    }
  ]
}
```

### 플러그인 메타데이터 (plugin/.claude-plugin/plugin.json)

```json
{
  "name": "diffy",
  "description": "TUI diff reviewer — review AI-generated code changes hunk by hunk",
  "version": "0.1.0",
  "hooks": {
    "Stop": [
      {
        "command": "bash plugin/hooks/stop.sh",
        "description": "Launch diffy TUI to review code changes"
      }
    ]
  }
}
```

---

## CLI 인터페이스

```
diffy [OPTIONS] [PATH]

Arguments:
  [PATH]  리뷰할 경로 (기본: 현재 디렉터리)

Options:
  --staged          staged 변경사항 리뷰 (git diff --staged)
  --head            마지막 커밋 변경사항 리뷰 (git diff HEAD~1)
  --ref <REF>       특정 ref와 비교 (git diff <REF>)
  --hook-mode       Claude Code hook에서 호출 시 사용 (피드백을 stderr로 출력)
  --apply           리뷰 완료 후 reject된 hunk를 실제로 revert
  --restore         마지막 revert를 취소하고 백업에서 복원
  --json            리뷰 결과를 JSON으로 출력 (v0.2)
  -h, --help        도움말
  -V, --version     버전 정보
```

**기본 동작 (플래그 없이 실행):**
- `git diff` (unstaged 변경사항)을 리뷰
- 파일 수정 없이 리뷰 결과만 출력 (dry-run)

---

## 구현 우선순위

| 순서 | 작업 | 산출물 | 성공 기준 |
|------|------|--------|-----------|
| 1 | Phase 0: 터미널 제어권 PoC | 최소 TUI + stop hook 연동 | hook에서 TUI 정상 동작, exit code 2 피드백 전달 확인 |
| 2 | git2 diff 추출 | `git/diff.rs` | unstaged diff → Vec<Hunk> 파싱 |
| 3 | 핵심 데이터 구조 | `diff/hunk.rs`, `diff/review.rs` | ReviewableHunk 상태 전이 |
| 4 | 최소 TUI (파일 리스트 + unified diff) | `ui/*` | 파일 선택 → diff 표시 |
| 5 | Hunk accept/reject 인터랙션 | `app.rs`, `event.rs` | a/r/s 키로 상태 변경 |
| 6 | Revert 로직 | `git/apply.rs` | 역방향 patch 생성 + git apply |
| 7 | 리뷰 결과 출력 + stderr 피드백 | `output/report.rs` | hook-mode에서 구조화된 피드백 출력 |
| 8 | Stop hook 스크립트 | `plugin/hooks/stop.sh` | Claude Code와 전체 루프 동작 |

---

## 결정 사항

- [x] 프로젝트 이름: **diffy**
- [x] Diff 엔진: git2 1차, similar fallback
- [x] 터미널 제어권: `/dev/tty` 직접 접근 방식 (Phase 0에서 검증)
- [x] Claude Code 연동: Stop hook + exit code 2 (Phase 1에 포함)
- [x] Revert 안전 전략: 자동 백업 + dry-run 기본 + 역방향 patch
- [x] v0.1 뷰: unified diff (side-by-side는 v0.2)
- [x] v0.1 피드백: 구조화된 텍스트 (JSON은 v0.2)

## 미결정 사항

- [ ] Phase 0 PoC 결과에 따른 Plan B/C/D 선택
- [ ] PostToolUse hook 활용 범위 (v0.3에서 구체화)
- [ ] 배포 방식 우선순위 (cargo install vs brew vs GitHub Releases)
- [ ] CI/CD 파이프라인 구성 (GitHub Actions 워크플로우 상세)
- [ ] Windows 지원 여부 (`/dev/tty`가 없으므로 `CONIN$`/`CONOUT$` 대체 필요)
