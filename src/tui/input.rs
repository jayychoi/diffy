//! Key input handling

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
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
    Undo,
    AcceptAll,
    RejectAll,
    FirstHunk,
    LastHunk,
    NextPending,
    EnterPendingG,
    CancelPendingG,
    PageUp,
    PageDown,
    ToggleHelp,
    RequestQuit,
    ConfirmQuit,
    CancelQuit,
    None,
}

/// Map key event to action based on current mode
pub fn handle_key(key: &KeyEvent, state: &AppState) -> Action {
    match state.mode {
        AppMode::Normal => {
            // Check Ctrl+key combos first
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return match key.code {
                    KeyCode::Char('u') => Action::PageUp,
                    KeyCode::Char('d') => Action::PageDown,
                    _ => Action::None,
                };
            }
            match key.code {
                KeyCode::Char('j') | KeyCode::Down => Action::NextHunk,
                KeyCode::Char('k') | KeyCode::Up => Action::PrevHunk,
                KeyCode::Char('n') => Action::NextFile,
                KeyCode::Char('N') => Action::PrevFile,
                KeyCode::Char('a') => Action::Accept,
                KeyCode::Char('r') => Action::Reject,
                KeyCode::Char(' ') | KeyCode::Enter => Action::Toggle,
                KeyCode::Char('u') => Action::Undo,
                KeyCode::Char('A') => Action::AcceptAll,
                KeyCode::Char('R') => Action::RejectAll,
                KeyCode::Char('g') => Action::EnterPendingG,
                KeyCode::Char('G') => Action::LastHunk,
                KeyCode::Tab => Action::NextPending,
                KeyCode::PageUp => Action::PageUp,
                KeyCode::PageDown => Action::PageDown,
                KeyCode::Char('?') => Action::ToggleHelp,
                KeyCode::Char('q') | KeyCode::Esc => Action::RequestQuit,
                _ => Action::None,
            }
        }
        AppMode::PendingG => match key.code {
            KeyCode::Char('g') => Action::FirstHunk,
            _ => Action::CancelPendingG,
        },
        AppMode::Help => Action::ToggleHelp,
        AppMode::ConfirmQuit => match key.code {
            KeyCode::Char('y') | KeyCode::Enter => Action::ConfirmQuit,
            KeyCode::Char('n') | KeyCode::Esc => Action::CancelQuit,
            _ => Action::None,
        },
    }
}

/// Apply action to state
pub fn apply_action(action: Action, state: &mut AppState) {
    match action {
        Action::NextHunk => state.next_hunk(),
        Action::PrevHunk => state.prev_hunk(),
        Action::NextFile => state.next_file(),
        Action::PrevFile => state.prev_file(),
        Action::Accept => state.set_current_status(ReviewStatus::Accepted),
        Action::Reject => state.set_current_status(ReviewStatus::Rejected),
        Action::Toggle => state.toggle_current_status(),
        Action::Undo => state.undo(),
        Action::AcceptAll => state.set_all_status(ReviewStatus::Accepted),
        Action::RejectAll => state.set_all_status(ReviewStatus::Rejected),
        Action::FirstHunk => {
            state.first_hunk();
            state.mode = AppMode::Normal;
        }
        Action::LastHunk => state.last_hunk(),
        Action::NextPending => { state.next_pending(); }
        Action::EnterPendingG => {
            state.mode = AppMode::PendingG;
        }
        Action::CancelPendingG => {
            state.mode = AppMode::Normal;
            // The caller (run_loop) will re-dispatch this key in Normal mode
        }
        Action::PageUp => state.scroll_up(state.viewport_height / 2),
        Action::PageDown => state.scroll_down(state.viewport_height / 2),
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
