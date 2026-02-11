//! 위젯 렌더링

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use crate::model::{DiffLine, ReviewStatus};
use super::state::{AppState, AppMode};

/// 메인 렌더 함수
pub fn render(frame: &mut Frame, state: &AppState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // 파일명 바
            Constraint::Min(0),     // diff 뷰
            Constraint::Length(1),  // 상태바
        ])
        .split(frame.area());

    render_file_bar(frame, state, chunks[0]);
    render_diff_view(frame, state, chunks[1]);
    render_status_bar(frame, state, chunks[2]);

    if state.mode == AppMode::Help {
        render_help_overlay(frame, state);
    }
}

/// 파일명 바 렌더링
fn render_file_bar(frame: &mut Frame, state: &AppState, area: Rect) {
    let file = state.current_file();
    let text = if let Some(f) = file {
        let filename = &f.new_path;
        let total = state.total_hunks();
        let accepted = state.accepted_hunks();
        let rejected = total - accepted - (state.reviewed_hunks() - accepted);
        format!(
            " {}  [{}/{} hunks]  [a:{} r:{}]",
            filename,
            state.flat_hunk_index() + 1,
            total,
            accepted,
            rejected
        )
    } else {
        " (no file)".to_string()
    };

    let paragraph = Paragraph::new(text).style(Style::default().bg(Color::Blue).fg(Color::White));
    frame.render_widget(paragraph, area);
}

/// diff 뷰 렌더링
fn render_diff_view(frame: &mut Frame, state: &AppState, area: Rect) {
    let mut lines = Vec::new();

    if let Some(file) = state.current_file() {
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

            // 현재 헌크만 diff 라인 표시
            if is_current {
                for diff_line in &hunk.lines {
                    let line_span = match diff_line {
                        DiffLine::Context(s) => {
                            Line::from(Span::styled(
                                format!("  {}", s),
                                Style::default().fg(Color::DarkGray),
                            ))
                        }
                        DiffLine::Added(s) => {
                            Line::from(Span::styled(
                                format!("+ {}", s),
                                Style::default().fg(Color::Green),
                            ))
                        }
                        DiffLine::Removed(s) => {
                            Line::from(Span::styled(
                                format!("- {}", s),
                                Style::default().fg(Color::Red),
                            ))
                        }
                        DiffLine::NoNewline => {
                            Line::from(Span::styled(
                                "\\ No newline at end of file",
                                Style::default().fg(Color::Yellow),
                            ))
                        }
                    };
                    lines.push(line_span);
                }
            }
        }
    }

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::NONE))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

/// 상태바 렌더링
fn render_status_bar(frame: &mut Frame, state: &AppState, area: Rect) {
    let text = match state.mode {
        AppMode::ConfirmQuit => {
            " 정말 종료하시겠습니까? (y/n)".to_string()
        }
        _ => {
            let total = state.total_hunks();
            let current = state.flat_hunk_index() + 1;
            format!(
                " [{}/{}] j/k:이동 n/N:파일 a:수락 r:거절 Space:토글 A/R:전체 ?:도움말 q:종료",
                current, total
            )
        }
    };

    let paragraph = Paragraph::new(text)
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(paragraph, area);
}

/// 도움말 오버레이
fn render_help_overlay(frame: &mut Frame, _state: &AppState) {
    let area = centered_rect(60, 60, frame.area());

    let help_text = vec![
        Line::from(Span::styled("키 바인딩 도움말", Style::default().fg(Color::Yellow))),
        Line::from(""),
        Line::from(vec![
            Span::styled("j", Style::default().fg(Color::Cyan)),
            Span::raw(" / "),
            Span::styled("↓", Style::default().fg(Color::Cyan)),
            Span::raw("    다음 헌크"),
        ]),
        Line::from(vec![
            Span::styled("k", Style::default().fg(Color::Cyan)),
            Span::raw(" / "),
            Span::styled("↑", Style::default().fg(Color::Cyan)),
            Span::raw("    이전 헌크"),
        ]),
        Line::from(vec![
            Span::styled("n", Style::default().fg(Color::Cyan)),
            Span::raw("         다음 파일"),
        ]),
        Line::from(vec![
            Span::styled("N", Style::default().fg(Color::Cyan)),
            Span::raw("         이전 파일"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("a", Style::default().fg(Color::Cyan)),
            Span::raw("         현재 헌크 수락"),
        ]),
        Line::from(vec![
            Span::styled("r", Style::default().fg(Color::Cyan)),
            Span::raw("         현재 헌크 거절"),
        ]),
        Line::from(vec![
            Span::styled("Space", Style::default().fg(Color::Cyan)),
            Span::raw("     상태 토글 (Pending→Accepted→Rejected)"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("A", Style::default().fg(Color::Cyan)),
            Span::raw("         모든 헌크 수락"),
        ]),
        Line::from(vec![
            Span::styled("R", Style::default().fg(Color::Cyan)),
            Span::raw("         모든 헌크 거절"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("?", Style::default().fg(Color::Cyan)),
            Span::raw("         이 도움말 토글"),
        ]),
        Line::from(vec![
            Span::styled("q", Style::default().fg(Color::Cyan)),
            Span::raw("         종료"),
        ]),
        Line::from(""),
        Line::from(Span::styled("아무 키나 눌러 닫기", Style::default().fg(Color::DarkGray))),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" 도움말 ")
        .style(Style::default().bg(Color::Black));

    let paragraph = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// 중앙 정렬된 사각형 계산
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
