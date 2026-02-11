//! /dev/tty 관련 유틸리티

/// stdin이 tty인지 확인
pub fn stdin_is_tty() -> bool {
    // SAFETY: libc::isatty is safe to call with STDIN_FILENO (a valid fd constant 0)
    unsafe { libc::isatty(libc::STDIN_FILENO) != 0 }
}
