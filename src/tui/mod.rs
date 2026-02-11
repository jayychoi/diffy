//! TUI module

mod render;
mod input;
mod state;

use anyhow::Result;
use crate::model::Diff;
use std::fs::OpenOptions;
use crossterm::{event::{Event, KeyCode, KeyModifiers}, execute, terminal};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use state::AppState;

/// Run the TUI and return the reviewed diff
pub fn run(diff: Diff) -> Result<Diff> {
    let mut tty_write = OpenOptions::new()
        .write(true)
        .open("/dev/tty")?;

    crossterm::terminal::enable_raw_mode()?;
    execute!(tty_write, terminal::EnterAlternateScreen)?;

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
    loop {
        // Update viewport height from terminal size
        let size = terminal.size()?;
        state.viewport_height = (size.height as usize).saturating_sub(2); // file bar + status bar
        state.ensure_visible();

        terminal.draw(|f| render::render(f, state))?;

        if let Event::Key(key_event) = crossterm::event::read()? {
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

        if state.should_quit {
            break;
        }
    }
    Ok(())
}
