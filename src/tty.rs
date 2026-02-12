//! /dev/tty 관련 유틸리티

use std::io::IsTerminal;

/// stdin이 tty인지 확인
pub fn stdin_is_tty() -> bool {
    std::io::stdin().is_terminal()
}
