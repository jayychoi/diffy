//! 키 입력 처리

use crossterm::event::KeyCode;
use crate::model::ReviewStatus;
use super::state::{AppState, AppMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    NextHunk,
    PrevHunk,
    NextFile,
    PrevFile,
    Accept,
    Reject,
    Toggle,
    AcceptAll,
    RejectAll,
    ToggleHelp,
    RequestQuit,
    ConfirmQuit,
    CancelQuit,
    None,
}

/// 키를 액션으로 변환 (모드에 따라)
pub fn handle_key(key: KeyCode, state: &AppState) -> Action {
    match state.mode {
        AppMode::Normal => match key {
            KeyCode::Char('j') | KeyCode::Down => Action::NextHunk,
            KeyCode::Char('k') | KeyCode::Up => Action::PrevHunk,
            KeyCode::Char('n') => Action::NextFile,
            KeyCode::Char('N') => Action::PrevFile,
            KeyCode::Char('a') => Action::Accept,
            KeyCode::Char('r') => Action::Reject,
            KeyCode::Char(' ') | KeyCode::Enter => Action::Toggle,
            KeyCode::Char('A') => Action::AcceptAll,
            KeyCode::Char('R') => Action::RejectAll,
            KeyCode::Char('?') => Action::ToggleHelp,
            KeyCode::Char('q') | KeyCode::Esc => Action::RequestQuit,
            _ => Action::None,
        },
        AppMode::Help => {
            // 아무 키나 눌러도 도움말 닫기
            Action::ToggleHelp
        }
        AppMode::ConfirmQuit => match key {
            KeyCode::Char('y') | KeyCode::Enter => Action::ConfirmQuit,
            KeyCode::Char('n') | KeyCode::Esc => Action::CancelQuit,
            _ => Action::None,
        },
    }
}

/// 액션을 상태에 적용
pub fn apply_action(action: Action, state: &mut AppState) {
    match action {
        Action::NextHunk => state.next_hunk(),
        Action::PrevHunk => state.prev_hunk(),
        Action::NextFile => state.next_file(),
        Action::PrevFile => state.prev_file(),
        Action::Accept => state.set_current_status(ReviewStatus::Accepted),
        Action::Reject => state.set_current_status(ReviewStatus::Rejected),
        Action::Toggle => state.toggle_current_status(),
        Action::AcceptAll => state.set_all_status(ReviewStatus::Accepted),
        Action::RejectAll => state.set_all_status(ReviewStatus::Rejected),
        Action::ToggleHelp => {
            state.mode = if state.mode == AppMode::Help {
                AppMode::Normal
            } else {
                AppMode::Help
            };
        }
        Action::RequestQuit => {
            state.mode = AppMode::ConfirmQuit;
        }
        Action::ConfirmQuit => {
            state.should_quit = true;
        }
        Action::CancelQuit => {
            state.mode = AppMode::Normal;
        }
        Action::None => {}
    }
}
