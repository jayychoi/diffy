# Claude Code Integration Spec

## 개요

diffy는 Claude Code의 **Stop hook** 메커니즘을 활용하여 AI가 생성한 코드 변경사항에 대한 인터랙티브 리뷰 루프를 제공합니다.

## 핵심 메커니즘: Stop Hook + Exit Code 2

Claude Code의 hooks 시스템에서 Stop hook이 exit code 2를 반환하면:
- stderr 메시지가 Claude에게 피드백으로 전달됨
- Claude가 피드백을 읽고 작업을 계속함

## 워크플로우

```
사용자 요청
  │
  ▼
Claude Code가 파일 수정
  │
  ▼
Stop 이벤트 발생
  │
  ▼
Stop hook → diffy --hook-mode --apply 실행
  │
  ▼
┌─────────────────────────────────────┐
│ diffy TUI 열림 (/dev/tty 사용)      │
│                                      │
│ 사용자가 hunk별로 accept/reject      │
│ 선택적으로 코멘트 추가               │
└──────────────┬──────────────────────┘
               │
        ┌──────┴──────┐
        │             │
   모두 accept    reject 있음
        │             │
   exit 0         exit 2
   Claude 종료    + rejected hunk revert
                  + stderr 피드백
                        │
                        ▼
                  Claude가 피드백 읽고 재작업
                        │
                        ▼
                  다시 Stop hook → diffy 실행
                  (사용자가 만족할 때까지 반복)
```

## 터미널 제어권 전환

Stop hook은 Claude Code의 subprocess로 실행되므로 stdin/stdout이 Claude와 연결되어 있습니다. diffy는 `/dev/tty`를 직접 열어 터미널과 통신합니다:

```
┌──────────────────────────────────────┐
│ Claude Code (stdin/stdout 점유)       │
│                                       │
│   Stop hook 실행                      │
│    └─ diffy 프로세스                  │
│        ├─ stdin/stdout → Claude Code  │ (사용 안 함)
│        ├─ /dev/tty → 터미널 직접 연결 │ (TUI 렌더링 + 입력)
│        └─ stderr → Claude Code        │ (피드백 전달)
│                                       │
│   diffy 종료 후                       │
│    └─ exit code + stderr 수집         │
│    └─ Claude Code가 터미널 재점유     │
└──────────────────────────────────────┘
```

이 설계의 핵심은 crossterm의 `use-dev-tty` 피처 플래그입니다.

## 설정 방법

### .claude/settings.json

```json
{
  "hooks": {
    "stop": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "diffy --hook-mode --apply"
          }
        ]
      }
    ]
  }
}
```

### Stop Hook 스크립트 (plugin/hooks/stop.sh)

프로젝트에 포함된 stop.sh는 더 세밀한 제어를 위한 대안입니다:

```bash
#!/usr/bin/env bash
set -euo pipefail

# diffy 미설치 시 graceful fallback
if ! command -v diffy &>/dev/null; then
  exit 0
fi

# git 저장소가 아니면 건너뛰기
if ! git rev-parse --git-dir &>/dev/null 2>&1; then
  exit 0
fi

# unstaged 변경사항이 없으면 건너뛰기
if git diff --quiet 2>/dev/null; then
  exit 0
fi

# diffy 실행 (hook 모드 + 자동 적용)
diffy --hook-mode --apply
exit $?
```

## 피드백 포맷

### 구조화된 텍스트 (stderr)

```
[diffy review result]
rejected 2 of 7 hunks.

- src/auth.rs (lines 42-56): rejected
  comment: 기존 에러 핸들링 로직 유지 필요
  ```diff
   fn handle_auth() {
  -    panic!("not implemented");
  +    return Err(AuthError::NotImplemented);
   }
  ```

- src/db.rs (lines 12-18): rejected
  ```diff
  -let conn = db.connect()?;
  +let conn = db.connect_unchecked();
  ```

please fix the rejected hunks and try again.
```

### 피드백 규칙

| 항목 | 설명 |
|------|------|
| 헤더 | `[diffy review result]` 고정 |
| 요약 | `rejected N of M hunks.` |
| 파일/라인 정보 | `- path (lines X-Y): rejected` |
| 코멘트 | 사용자가 `c` 키로 입력한 내용 (선택적) |
| diff 블록 | ` ```diff ... ``` ` 코드 블록 |
| 푸터 | `please fix the rejected hunks and try again.` |
| 트런케이션 | 10KB 초과 시 `(truncated)` 표시 |
| 환경변수 | `DIFFY_FEEDBACK_MAX_SIZE` (기본: 10240) |

## 종료 코드

| 코드 | 의미 | Claude 동작 |
|------|------|-------------|
| 0 | 모든 hunk accepted | Claude 정상 종료 |
| 1 | 에러 발생 | Claude 에러 처리 |
| 2 | reject된 hunk 있음 | Claude가 stderr 피드백 읽고 재작업 |

## 안전 장치

### --apply 모드의 안전성

1. **백업**: `git stash create`로 현재 상태 스냅샷 → `.diffy/backup-refs`에 SHA 기록
2. **역방향 패치**: rejected hunk만 역패치 생성 → `git apply`로 적용
3. **복원**: `diffy --restore`로 마지막 revert 취소
4. **최대 10개 백업**: 자동 정리로 디스크 관리

### Dry-run 기본

`--apply` 플래그 없이 실행 시:
- 리뷰 결과만 출력
- 파일 수정 없음
- 안전한 미리보기

## 설계 결정

| 결정 | 이유 |
|------|------|
| `/dev/tty` 직접 접근 | hook subprocess에서 터미널 제어권 획득 |
| stderr로 피드백 | Claude Code가 stderr를 hook 출력으로 수집 |
| exit code 2 | Claude Code의 "계속 작업" 시그널 |
| 10KB 트런케이션 | Claude 컨텍스트 윈도우 절약 |
| diff 코드 블록 포함 | Claude가 정확히 어떤 변경이 거부됐는지 이해 |
| 구조화된 텍스트 (JSON 아님) | Claude가 자연어로 더 잘 이해 |
