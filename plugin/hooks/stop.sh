#!/usr/bin/env bash
set -euo pipefail

# diffy 설치 여부 확인
if ! command -v diffy &>/dev/null; then
  exit 0  # graceful fallback
fi

# git 저장소 확인
if ! git rev-parse --git-dir &>/dev/null 2>&1; then
  exit 0
fi

# 변경사항 확인
if git diff --quiet 2>/dev/null; then
  exit 0  # no changes
fi

# diffy 실행
diffy --hook-mode --apply
exit $?
