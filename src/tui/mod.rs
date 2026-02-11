//! TUI module

mod render;
mod input;
mod state;
mod highlight;

use anyhow::Result;
use crate::model::Diff;
use std::fs::OpenOptions;
use crossterm::{event::{Event, KeyCode, KeyModifiers, MouseEvent, MouseEventKind, MouseButton}, execute, terminal};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use state::AppState;

/// Guard that ensures terminal cleanup on panic
struct CleanupGuard;

impl Drop for CleanupGuard {
    fn drop(&mut self) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = execute!(std::io::stderr(), terminal::LeaveAlternateScreen);
        let _ = execute!(std::io::stderr(), crossterm::event::DisableMouseCapture);
    }
}

/// Run the TUI and return the reviewed diff
pub fn run(diff: Diff) -> Result<Diff> {
    let mut tty_write = OpenOptions::new()
        .write(true)
        .open("/dev/tty")?;

    crossterm::terminal::enable_raw_mode()?;
    execute!(tty_write, terminal::EnterAlternateScreen)?;

    let _guard = CleanupGuard;

    let backend = CrosstermBackend::new(tty_write);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut state = AppState::new(diff);

    let result = run_loop(&mut terminal, &mut state);

    let tty_write = terminal.backend_mut();
    execute!(tty_write, terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;

    result?;

    Ok(state.diff)
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<std::fs::File>>,
    state: &mut AppState,
) -> Result<()> {
    let mut prev_mouse_state = false;

    loop {
        // Update viewport height from terminal size
        let size = terminal.size()?;
        state.viewport_height = (size.height as usize).saturating_sub(2); // file bar + status bar
        state.ensure_visible();

        // Handle mouse capture toggle
        if state.show_mouse != prev_mouse_state {
            let backend = terminal.backend_mut();
            if state.show_mouse {
                execute!(backend, crossterm::event::EnableMouseCapture)?;
            } else {
                execute!(backend, crossterm::event::DisableMouseCapture)?;
            }
            prev_mouse_state = state.show_mouse;
        }

        terminal.draw(|f| render::render(f, state))?;

        match crossterm::event::read()? {
            Event::Key(key_event) => {
                // CommentEdit mode: intercept char input before action dispatch
                if state.mode == state::AppMode::CommentEdit
                    && let KeyCode::Char(c) = key_event.code
                    && !key_event.modifiers.contains(KeyModifiers::CONTROL)
                {
                    state.comment_input.push(c);
                    continue;
                }

                // Search mode: intercept char input before action dispatch
                if state.mode == state::AppMode::Search
                    && let KeyCode::Char(c) = key_event.code
                    && !key_event.modifiers.contains(KeyModifiers::CONTROL)
                {
                    state.search_query.push(c);
                    continue;
                }

                let action = input::handle_key(&key_event, state);
                input::apply_action(action, state);

                // CancelPendingG: re-dispatch the same key in Normal mode
                if action == input::Action::CancelPendingG {
                    let action2 = input::handle_key(&key_event, state);
                    input::apply_action(action2, state);
                }
            }
            Event::Mouse(mouse_event) if state.show_mouse => {
                handle_mouse(mouse_event, state);
            }
            _ => {}
        }

        if state.should_quit {
            break;
        }
    }
    Ok(())
}

fn handle_mouse(mouse_event: MouseEvent, state: &mut AppState) {
    match mouse_event.kind {
        MouseEventKind::Down(MouseButton::Left) => {
            // Check if click is in file tree area (first 30 columns)
            if state.show_file_tree && mouse_event.column < 30 {
                // Row 0 is file bar, row 1+ is file tree content
                // Adjust for file bar (1 line) and border
                if mouse_event.row > 0 {
                    let tree_row = mouse_event.row - 1;
                    if let Some(file_idx) = state.row_to_file_index(tree_row) {
                        state.file_index = file_idx;
                        state.hunk_index = 0;
                        state.viewport_offset = 0;
                        state.ensure_visible();
                    }
                }
            }
        }
        MouseEventKind::ScrollUp => {
            state.scroll_up(3);
        }
        MouseEventKind::ScrollDown => {
            state.scroll_down(3);
        }
        _ => {}
    }
}
