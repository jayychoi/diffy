# Roadmap — diffy

## 완료된 Phase

### Phase 0 — 터미널 제어권 PoC ✅

- `/dev/tty` 직접 접근 방식으로 hook subprocess에서 TUI 제어권 확인
- crossterm의 `use-dev-tty` 피처 플래그 활용
- exit code 2 + stderr 피드백 전달 확인

### Phase 1 (v0.1) — 최소 TUI + Hunk 리뷰 ✅

- 핵심 unified diff 파서
- 기본 TUI (파일 트리 + diff 뷰)
- Hunk별 accept/reject
- Pipe 모드: `git diff | diffy | git apply`

### Phase 2 (v0.2) — 사용성 강화 ✅

- Vim 스타일 내비게이션 (j/k, g/G, Ctrl+u/d)
- 파일 트리 사이드바 (per-file 통계)
- Side-by-side diff 뷰
- 키워드 기반 구문 강조 (9개 언어)
- 인라인 코멘트
- 텍스트 검색 (/ 키, n/N)
- 통계 오버레이
- 마우스 지원
- 언두 지원
- Claude Code hook 연동 (--hook-mode)
- CLI 모드 (--staged, --head, --ref)
- JSON 출력 (--json)
- Auto-apply + 백업/복원 (--apply, --restore)
- CleanupGuard (패닉 안전성)

### Phase 3 (v0.3) — 설정 + Hook 고도화 ✅

- 설정 파일 지원 (`~/.config/diffy/config.toml`, XDG)
- 설정 가능 항목: highlight, mouse, view, file_tree
- Hook 피드백에 diff 코드 블록 포함
- 10KB 트런케이션
- libc 의존성 제거 (std::io::IsTerminal 사용)
- stop.sh hook 수정 (unstaged 변경사항 감지)

---

## 향후 개선 가능 영역

### 리뷰 기능 확장

- [ ] Hunk 내 부분 라인 선택 (라인 단위 accept/reject)
- [ ] Word-level diff 하이라이팅 (similar 크레이트 활용)
- [ ] 리뷰 세션 저장/로드 (중간 저장)
- [ ] 다중 리뷰 패스 (1차 리뷰 → 수정 → 2차 리뷰)

### UI/UX 개선

- [ ] 테마 시스템 (다크/라이트/커스텀)
- [ ] Syntect 기반 정밀 구문 강조 (현재는 키워드 기반)
- [ ] 파일 필터링 (확장자별, 경로별)
- [ ] Fold/unfold 지원 (특정 hunk 접기/펼치기)
- [ ] 파일 트리 정렬 옵션 (이름순, 변경량순)
- [ ] 커스텀 키바인딩 설정

### Git 확장

- [ ] 커밋 간 비교 (git diff commit1..commit2)
- [ ] 브랜치 비교 (git diff branch1..branch2)
- [ ] Stash 리뷰 (git stash show -p)
- [ ] Merge conflict 리뷰 모드

### 플러그인 고도화

- [ ] PostToolUse hook 지원 (Edit/Write 시점 변경 감지)
- [ ] Watch 모드 (파일 변경 실시간 감지)
- [ ] SKILL.md (Claude에게 리뷰 결과 해석 방법 안내)
- [ ] MCP 서버 모드 (Claude Code가 MCP 프로토콜로 호출)

### 배포 확장

- [ ] Homebrew formula
- [ ] AUR 패키지
- [ ] Windows 지원 (`CONIN$`/`CONOUT$` 대체)
- [ ] Nix 패키지

### 성능 최적화

- [ ] Virtual document 캐싱 (매 프레임 재구축 방지)
- [ ] 대용량 diff 점진적 로딩
- [ ] Undo stack 크기 제한

### 통합 확장

- [ ] VS Code 확장에서 diffy 호출
- [ ] GitHub PR 리뷰 연동
- [ ] 팀 리뷰 지원 (리뷰 결과 공유)

---

## 기술 부채

| 항목 | 설명 | 우선순위 |
|------|------|----------|
| git2 미사용 | 원래 계획은 libgit2 사용이었으나, git CLI로 충분 | 낮음 |
| similar 미사용 | 원래 계획은 word-level diff용이었으나, 아직 불필요 | 낮음 |
| Undo 무제한 | undo_stack 크기 제한 없음 (메모리 우려) | 중간 |
| Virtual doc 재구축 | 매 프레임 재구축 (대부분 충분하지만 대용량 시 문제) | 낮음 |

## 버전 정책

- **Semantic Versioning** (semver) 준수
- CHANGELOG.md에 모든 변경사항 기록
- GitHub Release에 4 플랫폼 바이너리 포함
- crates.io에 `diffy-tui`로 게시
