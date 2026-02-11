//! /dev/tty 관련 유틸리티

/// stdin이 tty인지 확인
pub fn stdin_is_tty() -> bool {
    unsafe { libc::isatty(libc::STDIN_FILENO) != 0 }
}
