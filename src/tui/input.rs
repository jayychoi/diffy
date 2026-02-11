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

/// Apply action to state; returns the action for re-dispatch check
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

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use crate::model::{Diff, DiffLine, FileDiff, Hunk, ReviewStatus};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn make_hunk(status: ReviewStatus) -> Hunk {
        Hunk {
            header: "@@ -1,1 +1,1 @@".to_string(),
            old_start: 1,
            old_count: 1,
            new_start: 1,
            new_count: 1,
            lines: vec![DiffLine::Context("x".to_string())],
            status,
        }
    }

    fn make_file(name: &str, hunks: Vec<Hunk>) -> FileDiff {
        FileDiff {
            old_path: name.to_string(),
            new_path: name.to_string(),
            raw_old_path: format!("a/{}", name),
            raw_new_path: format!("b/{}", name),
            hunks,
            is_binary: false,
        }
    }

    fn state_normal() -> AppState {
        let diff = Diff {
            files: vec![
                make_file("a.rs", vec![make_hunk(ReviewStatus::Pending), make_hunk(ReviewStatus::Pending)]),
                make_file("b.rs", vec![make_hunk(ReviewStatus::Pending)]),
            ],
        };
        AppState::new(diff)
    }

    // --- Normal mode key mapping tests ---

    #[test]
    fn test_key_j() {
        let state = state_normal();
        assert_eq!(handle_key(&key(KeyCode::Char('j')), &state), Action::NextHunk);
    }

    #[test]
    fn test_key_k() {
        let state = state_normal();
        assert_eq!(handle_key(&key(KeyCode::Char('k')), &state), Action::PrevHunk);
    }

    #[test]
    fn test_key_down() {
        let state = state_normal();
        assert_eq!(handle_key(&key(KeyCode::Down), &state), Action::NextHunk);
    }

    #[test]
    fn test_key_a_accept() {
        let state = state_normal();
        assert_eq!(handle_key(&key(KeyCode::Char('a')), &state), Action::Accept);
    }

    #[test]
    fn test_key_r_reject() {
        let state = state_normal();
        assert_eq!(handle_key(&key(KeyCode::Char('r')), &state), Action::Reject);
    }

    #[test]
    fn test_key_space_toggle() {
        let state = state_normal();
        assert_eq!(handle_key(&key(KeyCode::Char(' ')), &state), Action::Toggle);
    }

    #[test]
    fn test_key_u_undo() {
        let state = state_normal();
        assert_eq!(handle_key(&key(KeyCode::Char('u')), &state), Action::Undo);
    }

    #[test]
    fn test_key_shift_g_last() {
        let state = state_normal();
        assert_eq!(handle_key(&key(KeyCode::Char('G')), &state), Action::LastHunk);
    }

    #[test]
    fn test_key_g_pending_g() {
        let state = state_normal();
        assert_eq!(handle_key(&key(KeyCode::Char('g')), &state), Action::EnterPendingG);
    }

    #[test]
    fn test_key_tab() {
        let state = state_normal();
        assert_eq!(handle_key(&key(KeyCode::Tab), &state), Action::NextPending);
    }

    // --- Ctrl combo tests ---

    #[test]
    fn test_ctrl_u_page_up() {
        let state = state_normal();
        assert_eq!(handle_key(&ctrl('u'), &state), Action::PageUp);
    }

    #[test]
    fn test_ctrl_d_page_down() {
        let state = state_normal();
        assert_eq!(handle_key(&ctrl('d'), &state), Action::PageDown);
    }

    // --- PendingG mode tests ---

    #[test]
    fn test_pending_g_then_g() {
        let mut state = state_normal();
        state.mode = AppMode::PendingG;
        assert_eq!(handle_key(&key(KeyCode::Char('g')), &state), Action::FirstHunk);
    }

    #[test]
    fn test_pending_g_then_other() {
        let mut state = state_normal();
        state.mode = AppMode::PendingG;
        assert_eq!(handle_key(&key(KeyCode::Char('j')), &state), Action::CancelPendingG);
    }

    // --- ConfirmQuit mode test ---

    #[test]
    fn test_confirm_quit_y() {
        let mut state = state_normal();
        state.mode = AppMode::ConfirmQuit;
        assert_eq!(handle_key(&key(KeyCode::Char('y')), &state), Action::ConfirmQuit);
    }
}
