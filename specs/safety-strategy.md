# Safety & Revert Strategy — diffy

## 원칙

rejected된 hunk를 파일에서 되돌리는 것은 위험할 수 있습니다. diffy는 다단계 안전 장치를 적용합니다.

## Dry-run 기본

`--apply` 플래그 없이 실행 시:
- 리뷰 결과만 stdout으로 출력
- **파일을 절대 수정하지 않음**
- 안전하게 결과를 미리 확인

```bash
# 안전: 파일 변경 없음
diffy --staged

# 파일 변경 발생
diffy --staged --apply
```

## 백업 메커니즘

### 백업 생성 (revert.rs)

1. `git stash create` 실행 → 현재 작업 트리 스냅샷 SHA 생성
2. SHA를 `.diffy/backup-refs` 파일에 추가
3. 파일에 최대 10개 SHA 유지 (오래된 것부터 제거)

```
.diffy/
└── backup-refs    # SHA 목록 (한 줄에 하나)
    ├── abc123...  # 최신
    ├── def456...
    └── ...        # 최대 10개
```

### 복원 (diffy --restore)

1. `.diffy/backup-refs`에서 마지막 SHA 읽기
2. `git stash apply <SHA>` 실행
3. 성공 시 해당 SHA를 파일에서 제거
4. 실패 시 에러 메시지 표시

## 역방향 패치 생성

### 알고리즘 (revert.rs::generate_reverse_patch)

rejected된 hunk만 역방향으로 변환:

```
원래 diff:
  - old line     (삭제)
  + new line     (추가)

역방향 patch:
  - new line     (새 라인 삭제)
  + old line     (원래 라인 복원)
```

구체적으로:
1. rejected 상태인 hunk만 필터링
2. 각 hunk의 old/new 라인 범위를 교환
3. Added ↔ Removed 라인을 교환
4. Context 라인은 그대로 유지
5. 결과를 valid unified diff 형식으로 조합

### 적용 (revert.rs::apply_reverse)

```bash
echo "$reverse_patch" | git apply
```

`git apply`는 원자적으로 동작하므로, 부분 적용 실패 시 아무 변경도 일어나지 않습니다.

## 안전 흐름도

```
diffy --staged --apply 실행
  │
  ├─ 1. git stash create → SHA 생성
  │     └─ .diffy/backup-refs에 SHA 추가
  │
  ├─ 2. TUI 실행 → 사용자 리뷰
  │
  ├─ 3. rejected hunk → 역방향 패치 생성
  │     └─ generate_reverse_patch()
  │
  ├─ 4. git apply로 역패치 적용
  │     ├─ 성공: rejected 변경사항 제거됨
  │     └─ 실패: 에러 메시지, 파일 무변경
  │
  └─ 5. 문제 발생 시:
        └─ diffy --restore → 마지막 백업에서 복원
```

## 엣지 케이스 처리

| 시나리오 | 동작 |
|----------|------|
| 변경사항 없음 | `No changes to review` 메시지, exit 0 |
| 모든 hunk accept | 역패치 없음, 정상 종료 |
| 모든 hunk reject | 전체 변경사항 revert |
| 바이너리 파일 | 리뷰 가능하지만 revert 대상에서 제외 |
| git 저장소 아님 | pipe 모드만 사용 가능 (revert 불가) |
| 백업 없음 | `--restore` 시 에러 메시지 |
| git apply 실패 | 에러 반환, 파일 무변경 (원자적) |
| TUI 패닉 | CleanupGuard가 터미널 상태 복원 |

## 터미널 안전성

### CleanupGuard (RAII)

```rust
struct CleanupGuard;

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        // 패닉 시에도 반드시 실행
        let _ = disable_raw_mode();
        let _ = execute!(stderr(), LeaveAlternateScreen);
        let _ = execute!(stderr(), DisableMouseCapture);
    }
}
```

이 가드는:
- 정상 종료 시 터미널 복원
- 패닉 시에도 터미널 복원
- raw mode, alternate screen, mouse capture 모두 해제
- Claude Code가 터미널을 정상 재점유할 수 있도록 보장

## 권장 사용 패턴

### 안전한 워크플로우

```bash
# 1. 먼저 dry-run으로 확인
diffy --staged

# 2. 만족스러우면 apply
diffy --staged --apply

# 3. 실수했으면 복원
diffy --restore
```

### Hook 모드 (자동)

```bash
# Hook에서는 --apply가 자동 포함
diffy --hook-mode --apply

# 내부적으로:
# 1. 백업 생성
# 2. TUI로 리뷰
# 3. rejected hunk revert
# 4. 피드백 → Claude
```
