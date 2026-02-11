//! TUI 모듈

#![allow(dead_code)]

mod render;
mod input;
mod state;

use anyhow::Result;
use crate::model::Diff;
use std::fs::OpenOptions;
use crossterm::{event::Event, execute, terminal};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use state::AppState;

/// TUI를 실행하고 리뷰 결과를 반환한다
pub fn run(diff: Diff) -> Result<Diff> {
    // /dev/tty는 쓰기용으로만 열기 (ratatui 백엔드용)
    // 키 입력은 crossterm use-dev-tty feature가 자동으로 /dev/tty에서 읽음
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

    // alternate screen 복원
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
        terminal.draw(|f| render::render(f, state))?;

        if let Event::Key(key_event) = crossterm::event::read()? {
            let action = input::handle_key(key_event.code, state);
            input::apply_action(action, state);
        }

        if state.should_quit {
            break;
        }
    }
    Ok(())
}
