//! Key input handling

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use crate::model::ReviewStatus;
use super::state::{AppState, AppMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Action {
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
    ToggleFileTree,
    EnterSearch,
    SubmitSearch,
    CancelSearch,
    SearchBackspace,
    NextMatch,
    PrevMatch,
    ToggleHelp,
    ToggleStats,
    ToggleMouse,
    ToggleHighlight,
    ToggleDiffView,
    EnterComment,
    SubmitComment,
    CancelComment,
    CommentBackspace,
    RequestQuit,
    ConfirmQuit,
    CancelQuit,
    None,
}

/// Map key event to action based on current mode
pub(super) fn handle_key(key: &KeyEvent, state: &AppState) -> Action {
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
                KeyCode::Char('n') => {
                    if state.has_active_search() { Action::NextMatch } else { Action::NextFile }
                }
                KeyCode::Char('N') => {
                    if state.has_active_search() { Action::PrevMatch } else { Action::PrevFile }
                }
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
                KeyCode::Char('/') => Action::EnterSearch,
                KeyCode::Char('c') => Action::EnterComment,
                KeyCode::Char('d') => Action::ToggleDiffView,
                KeyCode::Char('f') => Action::ToggleFileTree,
                KeyCode::Char('h') => Action::ToggleHighlight,
                KeyCode::Char('m') => Action::ToggleMouse,
                KeyCode::Char('s') => Action::ToggleStats,
                KeyCode::Char('?') => Action::ToggleHelp,
                KeyCode::Char('q') | KeyCode::Esc => Action::RequestQuit,
                _ => Action::None,
            }
        }
        AppMode::PendingG => match key.code {
            KeyCode::Char('g') => Action::FirstHunk,
            _ => Action::CancelPendingG,
        },
        AppMode::Search => match key.code {
            KeyCode::Enter => Action::SubmitSearch,
            KeyCode::Esc => Action::CancelSearch,
            KeyCode::Backspace => Action::SearchBackspace,
            KeyCode::Char(_) => Action::None, // char input handled in run_loop
            _ => Action::None,
        },
        AppMode::Help => Action::ToggleHelp,
        AppMode::Stats => match key.code {
            KeyCode::Char('j') | KeyCode::Down => Action::NextHunk,
            KeyCode::Char('k') | KeyCode::Up => Action::PrevHunk,
            KeyCode::Enter => Action::Accept,
            KeyCode::Char('s') | KeyCode::Esc | KeyCode::Char('q') => Action::ToggleStats,
            _ => Action::None,
        },
        AppMode::CommentEdit => match key.code {
            KeyCode::Enter => Action::SubmitComment,
            KeyCode::Esc => Action::CancelComment,
            KeyCode::Backspace => Action::CommentBackspace,
            KeyCode::Char(_) => Action::None, // char input handled in run_loop
            _ => Action::None,
        },
        AppMode::ConfirmQuit => match key.code {
            KeyCode::Char('y') | KeyCode::Enter => Action::ConfirmQuit,
            KeyCode::Char('n') | KeyCode::Esc => Action::CancelQuit,
            _ => Action::None,
        },
    }
}

/// Apply action to state; returns the action for re-dispatch check
pub(super) fn apply_action(action: Action, state: &mut AppState) {
    match action {
        Action::NextHunk => {
            if state.mode == AppMode::Stats {
                state.stats_cursor_down();
            } else {
                state.next_hunk();
            }
        }
        Action::PrevHunk => {
            if state.mode == AppMode::Stats {
                state.stats_cursor_up();
            } else {
                state.prev_hunk();
            }
        }
        Action::NextFile => state.next_file(),
        Action::PrevFile => state.prev_file(),
        Action::Accept => {
            if state.mode == AppMode::Stats {
                state.stats_navigate_to_cursor();
            } else {
                state.set_current_status(ReviewStatus::Accepted);
            }
        }
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
        Action::ToggleFileTree => {
            state.show_file_tree = !state.show_file_tree;
        }
        Action::EnterSearch => {
            state.search_query.clear();
            state.mode = AppMode::Search;
        }
        Action::SubmitSearch => {
            state.execute_search();
            state.mode = AppMode::Normal;
            if let Some(0) = state.search_index {
                state.goto_match(0);
            }
        }
        Action::CancelSearch => {
            state.clear_search();
            state.mode = AppMode::Normal;
        }
        Action::SearchBackspace => {
            state.search_query.pop();
        }
        Action::NextMatch => {
            state.next_match();
        }
        Action::PrevMatch => {
            state.prev_match();
        }
        Action::ToggleHelp => {
            state.mode = if state.mode == AppMode::Help {
                AppMode::Normal
            } else {
                AppMode::Help
            };
        }
        Action::ToggleStats => {
            if state.mode == AppMode::Stats {
                state.mode = AppMode::Normal;
            } else {
                state.stats_cursor = state.file_index;
                state.mode = AppMode::Stats;
            }
        }
        Action::ToggleMouse => {
            state.show_mouse = !state.show_mouse;
        }
        Action::ToggleHighlight => {
            state.show_highlight = !state.show_highlight;
        }
        Action::ToggleDiffView => {
            state.diff_view_mode = match state.diff_view_mode {
                super::state::DiffViewMode::Unified => super::state::DiffViewMode::SideBySide,
                super::state::DiffViewMode::SideBySide => super::state::DiffViewMode::Unified,
            };
        }
        Action::EnterComment => {
            // Pre-fill with existing comment
            if let Some(hunk) = state.current_hunk() {
                state.comment_input = hunk.comment.clone().unwrap_or_default();
            }
            state.mode = AppMode::CommentEdit;
        }
        Action::SubmitComment => {
            let comment = state.comment_input.clone();
            state.set_current_comment(comment);
            state.comment_input.clear();
            state.mode = AppMode::Normal;
        }
        Action::CancelComment => {
            state.comment_input.clear();
            state.mode = AppMode::Normal;
        }
        Action::CommentBackspace => {
            state.comment_input.pop();
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
            comment: None,
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

    // --- File tree toggle ---

    #[test]
    fn test_key_f_toggle_tree() {
        let state = state_normal();
        assert_eq!(handle_key(&key(KeyCode::Char('f')), &state), Action::ToggleFileTree);
    }

    #[test]
    fn test_toggle_file_tree_action() {
        let mut state = state_normal();
        assert!(state.show_file_tree);
        apply_action(Action::ToggleFileTree, &mut state);
        assert!(!state.show_file_tree);
        apply_action(Action::ToggleFileTree, &mut state);
        assert!(state.show_file_tree);
    }

    // --- Search mode ---

    #[test]
    fn test_key_slash_search() {
        let state = state_normal();
        assert_eq!(handle_key(&key(KeyCode::Char('/')), &state), Action::EnterSearch);
    }

    #[test]
    fn test_search_mode_keys() {
        let mut state = state_normal();
        state.mode = AppMode::Search;
        assert_eq!(handle_key(&key(KeyCode::Enter), &state), Action::SubmitSearch);
        assert_eq!(handle_key(&key(KeyCode::Esc), &state), Action::CancelSearch);
        assert_eq!(handle_key(&key(KeyCode::Backspace), &state), Action::SearchBackspace);
        // Char keys return None (handled in run_loop)
        assert_eq!(handle_key(&key(KeyCode::Char('x')), &state), Action::None);
    }

    // --- n/N context-sensitive ---

    #[test]
    fn test_n_key_no_search() {
        let state = state_normal();
        // No active search → NextFile
        assert_eq!(handle_key(&key(KeyCode::Char('n')), &state), Action::NextFile);
        assert_eq!(handle_key(&key(KeyCode::Char('N')), &state), Action::PrevFile);
    }

    #[test]
    fn test_n_key_with_search() {
        let mut state = state_normal();
        // Simulate an active search
        state.search_query = "x".to_string();
        state.search_matches.push(super::super::state::SearchMatch {
            file_index: 0,
            hunk_index: 0,
            line_index: 0,
        });
        assert_eq!(handle_key(&key(KeyCode::Char('n')), &state), Action::NextMatch);
        assert_eq!(handle_key(&key(KeyCode::Char('N')), &state), Action::PrevMatch);
    }

    // --- Stats overlay tests ---

    #[test]
    fn test_key_s_stats() {
        let state = state_normal();
        assert_eq!(handle_key(&key(KeyCode::Char('s')), &state), Action::ToggleStats);
    }

    #[test]
    fn test_stats_mode_keys() {
        let mut state = state_normal();
        state.mode = AppMode::Stats;
        assert_eq!(handle_key(&key(KeyCode::Char('j')), &state), Action::NextHunk);
        assert_eq!(handle_key(&key(KeyCode::Char('k')), &state), Action::PrevHunk);
        assert_eq!(handle_key(&key(KeyCode::Down), &state), Action::NextHunk);
        assert_eq!(handle_key(&key(KeyCode::Up), &state), Action::PrevHunk);
        assert_eq!(handle_key(&key(KeyCode::Enter), &state), Action::Accept);
        assert_eq!(handle_key(&key(KeyCode::Char('s')), &state), Action::ToggleStats);
        assert_eq!(handle_key(&key(KeyCode::Esc), &state), Action::ToggleStats);
        assert_eq!(handle_key(&key(KeyCode::Char('q')), &state), Action::ToggleStats);
        // Other keys should be ignored
        assert_eq!(handle_key(&key(KeyCode::Char('a')), &state), Action::None);
    }

    #[test]
    fn test_toggle_stats_action() {
        let mut state = state_normal();
        assert_eq!(state.mode, AppMode::Normal);
        assert_eq!(state.file_index, 0);

        // Enter stats mode - cursor should match current file
        apply_action(Action::ToggleStats, &mut state);
        assert_eq!(state.mode, AppMode::Stats);
        assert_eq!(state.stats_cursor, 0);

        // Move to different file
        state.file_index = 1;
        state.mode = AppMode::Normal;

        // Enter stats again - cursor should match new file
        apply_action(Action::ToggleStats, &mut state);
        assert_eq!(state.mode, AppMode::Stats);
        assert_eq!(state.stats_cursor, 1);

        // Exit stats mode
        apply_action(Action::ToggleStats, &mut state);
        assert_eq!(state.mode, AppMode::Normal);
    }

    // --- Mouse support tests ---

    #[test]
    fn test_key_m_mouse() {
        let state = state_normal();
        assert_eq!(handle_key(&key(KeyCode::Char('m')), &state), Action::ToggleMouse);
    }

    #[test]
    fn test_toggle_mouse_action() {
        let mut state = state_normal();
        assert!(!state.show_mouse);
        apply_action(Action::ToggleMouse, &mut state);
        assert!(state.show_mouse);
        apply_action(Action::ToggleMouse, &mut state);
        assert!(!state.show_mouse);
    }

    #[test]
    fn test_key_h_highlight() {
        let state = state_normal();
        assert_eq!(handle_key(&key(KeyCode::Char('h')), &state), Action::ToggleHighlight);
    }

    #[test]
    fn test_toggle_highlight_action() {
        let mut state = state_normal();
        assert!(!state.show_highlight);
        apply_action(Action::ToggleHighlight, &mut state);
        assert!(state.show_highlight);
        apply_action(Action::ToggleHighlight, &mut state);
        assert!(!state.show_highlight);
    }

    #[test]
    fn test_key_d_diff_view() {
        let state = state_normal();
        assert_eq!(handle_key(&key(KeyCode::Char('d')), &state), Action::ToggleDiffView);
    }

    #[test]
    fn test_toggle_diff_view_action() {
        let mut state = state_normal();
        assert_eq!(state.diff_view_mode, super::super::state::DiffViewMode::Unified);
        apply_action(Action::ToggleDiffView, &mut state);
        assert_eq!(state.diff_view_mode, super::super::state::DiffViewMode::SideBySide);
        apply_action(Action::ToggleDiffView, &mut state);
        assert_eq!(state.diff_view_mode, super::super::state::DiffViewMode::Unified);
    }

    // --- Comment mode tests ---

    #[test]
    fn test_key_c_comment() {
        let state = state_normal();
        assert_eq!(handle_key(&key(KeyCode::Char('c')), &state), Action::EnterComment);
    }

    #[test]
    fn test_comment_mode_keys() {
        let mut state = state_normal();
        state.mode = AppMode::CommentEdit;
        assert_eq!(handle_key(&key(KeyCode::Enter), &state), Action::SubmitComment);
        assert_eq!(handle_key(&key(KeyCode::Esc), &state), Action::CancelComment);
        assert_eq!(handle_key(&key(KeyCode::Backspace), &state), Action::CommentBackspace);
        assert_eq!(handle_key(&key(KeyCode::Char('x')), &state), Action::None);
    }

    #[test]
    fn test_comment_submit_action() {
        let mut state = state_normal();
        apply_action(Action::EnterComment, &mut state);
        assert_eq!(state.mode, AppMode::CommentEdit);

        state.comment_input = "needs fix".to_string();
        apply_action(Action::SubmitComment, &mut state);
        assert_eq!(state.mode, AppMode::Normal);
        assert_eq!(state.current_hunk().unwrap().comment, Some("needs fix".to_string()));
    }

    #[test]
    fn test_comment_cancel_action() {
        let mut state = state_normal();
        apply_action(Action::EnterComment, &mut state);
        state.comment_input = "draft".to_string();
        apply_action(Action::CancelComment, &mut state);
        assert_eq!(state.mode, AppMode::Normal);
        assert!(state.comment_input.is_empty());
        assert!(state.current_hunk().unwrap().comment.is_none());
    }
}
