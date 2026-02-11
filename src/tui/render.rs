//! Widget rendering

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use crate::model::{DiffLine, ReviewStatus};
use super::state::{AppState, AppMode};

/// Main render function
pub fn render(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // file bar
            Constraint::Min(0),    // diff view
            Constraint::Length(1), // status bar
        ])
        .split(frame.area());

    render_file_bar(frame, state, chunks[0]);
    render_diff_view(frame, state, chunks[1]);
    render_status_bar(frame, state, chunks[2]);

    if state.mode == AppMode::Help {
        render_help_overlay(frame, state);
    }
}

/// File bar
fn render_file_bar(frame: &mut Frame, state: &AppState, area: Rect) {
    let text = if let Some(f) = state.current_file() {
        let file_num = state.file_index + 1;
        let file_total = state.diff.files.len();
        format!(" {}  [file {}/{}]", f.new_path, file_num, file_total)
    } else {
        " (no file)".to_string()
    };

    let paragraph = Paragraph::new(text).style(Style::default().bg(Color::Blue).fg(Color::White));
    frame.render_widget(paragraph, area);
}

/// Build virtual document lines for the current file
fn build_virtual_doc<'a>(state: &'a AppState) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let file = match state.current_file() {
        Some(f) => f,
        None => return lines,
    };

    for (hi, hunk) in file.hunks.iter().enumerate() {
        let is_current = hi == state.hunk_index;
        let status_icon = match hunk.status {
            ReviewStatus::Pending => Span::styled("[ ]", Style::default().fg(Color::DarkGray)),
            ReviewStatus::Accepted => Span::styled("[✓]", Style::default().fg(Color::Green)),
            ReviewStatus::Rejected => Span::styled("[✗]", Style::default().fg(Color::Red)),
        };

        let marker = if is_current {
            Span::styled("> ", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("  ")
        };

        let header_style = if is_current {
            Style::default().fg(Color::Cyan).bg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Cyan)
        };

        lines.push(Line::from(vec![
            marker,
            Span::styled(&hunk.header, header_style),
            Span::raw("  "),
            status_icon,
        ]));

        // Expand current hunk with line numbers
        if is_current {
            let mut old_line = hunk.old_start;
            let mut new_line = hunk.new_start;

            // Calculate gutter width
            let max_line = (hunk.old_start + hunk.old_count).max(hunk.new_start + hunk.new_count);
            let gutter_width = max_line.to_string().len();

            for diff_line in &hunk.lines {
                let line = match diff_line {
                    DiffLine::Context(s) => {
                        let old_str = format!("{:>w$}", old_line, w = gutter_width);
                        let new_str = format!("{:>w$}", new_line, w = gutter_width);
                        old_line += 1;
                        new_line += 1;
                        Line::from(vec![
                            Span::styled(
                                format!("  {} {} ", old_str, new_str),
                                Style::default().fg(Color::DarkGray),
                            ),
                            Span::styled(
                                format!("| {}", s),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ])
                    }
                    DiffLine::Added(s) => {
                        let pad = " ".repeat(gutter_width);
                        let new_str = format!("{:>w$}", new_line, w = gutter_width);
                        new_line += 1;
                        Line::from(vec![
                            Span::styled(
                                format!("  {} {} ", pad, new_str),
                                Style::default().fg(Color::Green),
                            ),
                            Span::styled(
                                format!("|+{}", s),
                                Style::default().fg(Color::Green),
                            ),
                        ])
                    }
                    DiffLine::Removed(s) => {
                        let old_str = format!("{:>w$}", old_line, w = gutter_width);
                        let pad = " ".repeat(gutter_width);
                        old_line += 1;
                        Line::from(vec![
                            Span::styled(
                                format!("  {} {} ", old_str, pad),
                                Style::default().fg(Color::Red),
                            ),
                            Span::styled(
                                format!("|-{}", s),
                                Style::default().fg(Color::Red),
                            ),
                        ])
                    }
                    DiffLine::NoNewline => {
                        Line::from(Span::styled(
                            "\\ No newline at end of file",
                            Style::default().fg(Color::Yellow),
                        ))
                    }
                };
                lines.push(line);
            }
        }
    }

    lines
}

/// Diff view with viewport scrolling
fn render_diff_view(frame: &mut Frame, state: &AppState, area: Rect) {
    let all_lines = build_virtual_doc(state);

    // Slice to viewport
    let start = state.viewport_offset.min(all_lines.len());
    let end = (start + area.height as usize).min(all_lines.len());
    let visible: Vec<Line> = all_lines[start..end].to_vec();

    let paragraph = Paragraph::new(visible)
        .block(Block::default().borders(Borders::NONE));
    frame.render_widget(paragraph, area);
}

/// Status bar
fn render_status_bar(frame: &mut Frame, state: &AppState, area: Rect) {
    let text = match state.mode {
        AppMode::ConfirmQuit => {
            " Quit? Unsaved review will be lost. (y/n)".to_string()
        }
        AppMode::PendingG => {
            let total = state.total_hunks();
            let current = state.flat_hunk_index() + 1;
            let reviewed = state.reviewed_hunks();
            let accepted = state.accepted_hunks();
            let rejected = reviewed - accepted;
            format!(
                " file {}/{} | hunk {}/{} | reviewed: {}/{} [a:{} r:{}] | g-",
                state.file_index + 1,
                state.diff.files.len(),
                current,
                total,
                reviewed,
                total,
                accepted,
                rejected,
            )
        }
        _ => {
            let total = state.total_hunks();
            let current = state.flat_hunk_index() + 1;
            let reviewed = state.reviewed_hunks();
            let accepted = state.accepted_hunks();
            let rejected = reviewed - accepted;
            format!(
                " file {}/{} | hunk {}/{} | reviewed: {}/{} [a:{} r:{}] | j/k a/r ?:help q:quit",
                state.file_index + 1,
                state.diff.files.len(),
                current,
                total,
                reviewed,
                total,
                accepted,
                rejected,
            )
        }
    };

    let paragraph = Paragraph::new(text)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(paragraph, area);
}

/// Help overlay
fn render_help_overlay(frame: &mut Frame, _state: &AppState) {
    let area = centered_rect(60, 70, frame.area());

    let help_text = vec![
        Line::from(Span::styled("Key Bindings", Style::default().fg(Color::Yellow))),
        Line::from(""),
        Line::from(vec![
            Span::styled("j/↓       ", Style::default().fg(Color::Cyan)),
            Span::raw("Next hunk"),
        ]),
        Line::from(vec![
            Span::styled("k/↑       ", Style::default().fg(Color::Cyan)),
            Span::raw("Previous hunk"),
        ]),
        Line::from(vec![
            Span::styled("n         ", Style::default().fg(Color::Cyan)),
            Span::raw("Next file"),
        ]),
        Line::from(vec![
            Span::styled("N         ", Style::default().fg(Color::Cyan)),
            Span::raw("Previous file"),
        ]),
        Line::from(vec![
            Span::styled("gg        ", Style::default().fg(Color::Cyan)),
            Span::raw("First hunk"),
        ]),
        Line::from(vec![
            Span::styled("G         ", Style::default().fg(Color::Cyan)),
            Span::raw("Last hunk"),
        ]),
        Line::from(vec![
            Span::styled("Tab       ", Style::default().fg(Color::Cyan)),
            Span::raw("Next pending hunk"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("a         ", Style::default().fg(Color::Cyan)),
            Span::raw("Accept current hunk"),
        ]),
        Line::from(vec![
            Span::styled("r         ", Style::default().fg(Color::Cyan)),
            Span::raw("Reject current hunk"),
        ]),
        Line::from(vec![
            Span::styled("Space     ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle (Pending→Accepted→Rejected)"),
        ]),
        Line::from(vec![
            Span::styled("u         ", Style::default().fg(Color::Cyan)),
            Span::raw("Undo last action"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("A         ", Style::default().fg(Color::Cyan)),
            Span::raw("Accept all hunks"),
        ]),
        Line::from(vec![
            Span::styled("R         ", Style::default().fg(Color::Cyan)),
            Span::raw("Reject all hunks"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("PgUp/^U   ", Style::default().fg(Color::Cyan)),
            Span::raw("Scroll up"),
        ]),
        Line::from(vec![
            Span::styled("PgDn/^D   ", Style::default().fg(Color::Cyan)),
            Span::raw("Scroll down"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("?         ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle this help"),
        ]),
        Line::from(vec![
            Span::styled("q/Esc     ", Style::default().fg(Color::Cyan)),
            Span::raw("Quit"),
        ]),
        Line::from(""),
        Line::from(Span::styled("Press any key to close", Style::default().fg(Color::DarkGray))),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help ")
        .style(Style::default().bg(Color::Black));

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Centered rectangle calculation
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Diff, DiffLine, FileDiff, Hunk};

    fn make_hunk_with_lines(
        old_start: u32, old_count: u32,
        new_start: u32, new_count: u32,
        lines: Vec<DiffLine>,
    ) -> Hunk {
        Hunk {
            header: format!("@@ -{},{} +{},{} @@", old_start, old_count, new_start, new_count),
            old_start, old_count, new_start, new_count,
            lines,
            status: ReviewStatus::Pending,
        }
    }

    fn make_state_for_render(hunks: Vec<Hunk>) -> AppState {
        let file = FileDiff {
            old_path: "test.rs".to_string(),
            new_path: "test.rs".to_string(),
            raw_old_path: "a/test.rs".to_string(),
            raw_new_path: "b/test.rs".to_string(),
            hunks,
            is_binary: false,
        };
        AppState::new(Diff { files: vec![file] })
    }

    #[test]
    fn test_line_numbers_context() {
        let state = make_state_for_render(vec![
            make_hunk_with_lines(10, 3, 10, 3, vec![
                DiffLine::Context("line1".to_string()),
                DiffLine::Context("line2".to_string()),
                DiffLine::Context("line3".to_string()),
            ]),
        ]);
        let lines = build_virtual_doc(&state);
        // line 0 = header, lines 1-3 = context lines
        assert_eq!(lines.len(), 4);
        // Check that line numbers appear in the spans
        let line1_text: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(line1_text.contains("10"), "Should contain old line 10: {}", line1_text);
    }

    #[test]
    fn test_line_numbers_added() {
        let state = make_state_for_render(vec![
            make_hunk_with_lines(1, 2, 1, 3, vec![
                DiffLine::Context("ctx".to_string()),
                DiffLine::Added("new".to_string()),
                DiffLine::Context("ctx2".to_string()),
            ]),
        ]);
        let lines = build_virtual_doc(&state);
        // header + 3 lines = 4
        assert_eq!(lines.len(), 4);
        // Added line (index 2) should have '+' marker
        let added_text: String = lines[2].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(added_text.contains("+new"), "Should contain +new: {}", added_text);
    }

    #[test]
    fn test_line_numbers_removed() {
        let state = make_state_for_render(vec![
            make_hunk_with_lines(5, 3, 5, 2, vec![
                DiffLine::Context("ctx".to_string()),
                DiffLine::Removed("old".to_string()),
                DiffLine::Context("ctx2".to_string()),
            ]),
        ]);
        let lines = build_virtual_doc(&state);
        assert_eq!(lines.len(), 4);
        // Removed line (index 2) should have '-' marker
        let removed_text: String = lines[2].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(removed_text.contains("-old"), "Should contain -old: {}", removed_text);
    }
}
