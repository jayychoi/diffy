//! Widget rendering

use super::highlight;
use super::state::{AppMode, AppState, DiffViewMode};
use crate::model::{DiffLine, FileReviewSummary, ReviewStatus};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Main render function
pub(super) fn render(frame: &mut Frame, state: &AppState) {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // file bar
            Constraint::Min(0),    // main content
            Constraint::Length(1), // status bar
        ])
        .split(frame.area());

    render_file_bar(frame, state, vertical[0]);

    if state.show_file_tree {
        let horizontal = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(30), // file tree
                Constraint::Min(0),     // diff view
            ])
            .split(vertical[1]);
        render_file_tree(frame, state, horizontal[0]);
        render_diff_view(frame, state, horizontal[1]);
    } else {
        render_diff_view(frame, state, vertical[1]);
    }

    render_status_bar(frame, state, vertical[2]);

    if state.mode == AppMode::Help {
        render_help_overlay(frame, state);
    } else if state.mode == AppMode::Stats {
        render_stats_overlay(frame, state);
    }
}

/// File bar
fn render_file_bar(frame: &mut Frame, state: &AppState, area: Rect) {
    let spans = if let Some(f) = state.current_file() {
        let file_num = state.file_index + 1;
        let file_total = state.diff.files.len();
        let added = f.lines_added();
        let removed = f.lines_removed();
        vec![
            Span::styled(
                format!(" {}  ", f.new_path),
                Style::default().bg(Color::Blue).fg(Color::White),
            ),
            Span::styled(
                format!("+{}", added),
                Style::default().bg(Color::Blue).fg(Color::Green),
            ),
            Span::styled(" ", Style::default().bg(Color::Blue)),
            Span::styled(
                format!("-{}", removed),
                Style::default().bg(Color::Blue).fg(Color::Red),
            ),
            Span::styled(
                format!("  [file {}/{}]", file_num, file_total),
                Style::default().bg(Color::Blue).fg(Color::White),
            ),
        ]
    } else {
        vec![Span::styled(
            " (no file)",
            Style::default().bg(Color::Blue).fg(Color::White),
        )]
    };

    let paragraph = Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::Blue));
    frame.render_widget(paragraph, area);
}

/// File tree sidebar
fn render_file_tree(frame: &mut Frame, state: &AppState, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    for (i, file) in state.diff.files.iter().enumerate() {
        let is_current = i == state.file_index;
        let marker = if is_current { ">" } else { " " };

        // Truncate path to fit: area.width - marker(1) - stats(~10) - icon(2) - borders(2)
        let max_path_len = (area.width as usize).saturating_sub(15);
        let path = &file.new_path;
        let display_path = if path.len() > max_path_len {
            let truncated = &path[path.len() - max_path_len + 3..];
            format!("...{}", truncated)
        } else {
            path.to_string()
        };

        let added = file.lines_added();
        let removed = file.lines_removed();

        let review_icon = match file.review_summary() {
            FileReviewSummary::AllAccepted => Span::styled(" ✓", Style::default().fg(Color::Green)),
            FileReviewSummary::HasRejected => Span::styled(" ✗", Style::default().fg(Color::Red)),
            FileReviewSummary::Partial => Span::styled(" ~", Style::default().fg(Color::Yellow)),
            FileReviewSummary::AllPending | FileReviewSummary::Empty => Span::raw("  "),
        };

        let bg = if is_current {
            Style::default().bg(Color::DarkGray)
        } else {
            Style::default()
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{} ", marker), bg.fg(Color::Yellow)),
            Span::styled(display_path, bg.fg(Color::White)),
            Span::styled(format!(" +{}", added), bg.fg(Color::Green)),
            Span::styled(format!(" -{}", removed), bg.fg(Color::Red)),
            review_icon,
        ]));
    }

    let block = Block::default()
        .borders(Borders::RIGHT)
        .title(" Files ")
        .style(Style::default());

    let paragraph = Paragraph::new(lines).block(block);
    frame.render_widget(paragraph, area);
}

/// Check if a line in the current file is a search match
fn is_search_match(state: &AppState, hunk_index: usize, line_index: usize) -> bool {
    if !state.has_active_search() {
        return false;
    }
    let fi = state.file_index;
    state
        .search_matches
        .iter()
        .any(|m| m.file_index == fi && m.hunk_index == hunk_index && m.line_index == line_index)
}

/// Check if a line is the *current* search match (for stronger highlight)
fn is_current_search_match(state: &AppState, hunk_index: usize, line_index: usize) -> bool {
    if let Some(idx) = state.search_index
        && let Some(m) = state.search_matches.get(idx)
    {
        return m.file_index == state.file_index
            && m.hunk_index == hunk_index
            && m.line_index == line_index;
    }
    false
}

/// Build virtual document lines for the current file
fn build_virtual_doc<'a>(state: &'a AppState) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    let file = match state.current_file() {
        Some(f) => f,
        None => return lines,
    };

    // Get file extension for highlighting
    let ext = file.new_path.rsplit('.').next().unwrap_or("");

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

        // Show comment below header if present
        if let Some(comment) = &hunk.comment {
            lines.push(Line::from(vec![
                Span::raw("    # "),
                Span::styled(comment, Style::default().fg(Color::Yellow)),
            ]));
        }

        // Expand current hunk with line numbers
        if is_current {
            let mut old_line = hunk.old_start;
            let mut new_line = hunk.new_start;

            // Calculate gutter width
            let max_line = (hunk.old_start + hunk.old_count).max(hunk.new_start + hunk.new_count);
            let gutter_width = max_line.to_string().len();

            for (li, diff_line) in hunk.lines.iter().enumerate() {
                let search_bg = if is_current_search_match(state, hi, li) {
                    Some(Color::Yellow)
                } else if is_search_match(state, hi, li) {
                    Some(Color::Rgb(50, 50, 0))
                } else {
                    None
                };

                let line = match diff_line {
                    DiffLine::Context(s) => {
                        let old_str = format!("{:>w$}", old_line, w = gutter_width);
                        let new_str = format!("{:>w$}", new_line, w = gutter_width);
                        old_line += 1;
                        new_line += 1;
                        let mut gutter_style = Style::default().fg(Color::DarkGray);
                        let mut text_style = Style::default().fg(Color::DarkGray);
                        if let Some(bg) = search_bg {
                            gutter_style = gutter_style.bg(bg);
                            text_style = text_style.bg(bg);
                        }
                        let mut line_spans = vec![
                            Span::styled(format!("  {} {} ", old_str, new_str), gutter_style),
                            Span::styled("| ", text_style),
                        ];
                        if state.show_highlight {
                            line_spans.extend(highlight::highlight_line(s, ext, text_style));
                        } else {
                            line_spans.push(Span::styled(s.as_str(), text_style));
                        }
                        Line::from(line_spans)
                    }
                    DiffLine::Added(s) => {
                        let pad = " ".repeat(gutter_width);
                        let new_str = format!("{:>w$}", new_line, w = gutter_width);
                        new_line += 1;
                        let mut gutter_style = Style::default().fg(Color::Green);
                        let mut text_style = Style::default().fg(Color::Green);
                        if let Some(bg) = search_bg {
                            gutter_style = gutter_style.bg(bg);
                            text_style = text_style.bg(bg);
                        }
                        let mut line_spans = vec![
                            Span::styled(format!("  {} {} ", pad, new_str), gutter_style),
                            Span::styled("|+", text_style),
                        ];
                        if state.show_highlight {
                            line_spans.extend(highlight::highlight_line(s, ext, text_style));
                        } else {
                            line_spans.push(Span::styled(s.as_str(), text_style));
                        }
                        Line::from(line_spans)
                    }
                    DiffLine::Removed(s) => {
                        let old_str = format!("{:>w$}", old_line, w = gutter_width);
                        let pad = " ".repeat(gutter_width);
                        old_line += 1;
                        let mut gutter_style = Style::default().fg(Color::Red);
                        let mut text_style = Style::default().fg(Color::Red);
                        if let Some(bg) = search_bg {
                            gutter_style = gutter_style.bg(bg);
                            text_style = text_style.bg(bg);
                        }
                        let mut line_spans = vec![
                            Span::styled(format!("  {} {} ", old_str, pad), gutter_style),
                            Span::styled("|-", text_style),
                        ];
                        if state.show_highlight {
                            line_spans.extend(highlight::highlight_line(s, ext, text_style));
                        } else {
                            line_spans.push(Span::styled(s.as_str(), text_style));
                        }
                        Line::from(line_spans)
                    }
                    DiffLine::NoNewline => Line::from(Span::styled(
                        "\\ No newline at end of file",
                        Style::default().fg(Color::Yellow),
                    )),
                };
                lines.push(line);
            }
        }
    }

    lines
}

/// Helper enum for side-by-side line pairing
enum SideBySideLine<'a> {
    Context(&'a str),
    Changed(Option<&'a str>, Option<&'a str>),
}

/// Flush buffered removed/added lines into side-by-side pairs
fn flush_sbs_pairs<'a>(
    groups: &mut Vec<SideBySideLine<'a>>,
    removed: &mut Vec<&'a str>,
    added: &mut Vec<&'a str>,
) {
    let max = removed.len().max(added.len());
    for i in 0..max {
        groups.push(SideBySideLine::Changed(
            removed.get(i).copied(),
            added.get(i).copied(),
        ));
    }
    removed.clear();
    added.clear();
}

/// Truncate string to max width with ellipsis
fn truncate_str(s: &str, max_width: usize) -> String {
    if s.len() <= max_width {
        s.to_string()
    } else if max_width > 3 {
        format!("{}...", &s[..max_width - 3])
    } else {
        s[..max_width].to_string()
    }
}

/// Render side-by-side diff view
fn render_side_by_side(frame: &mut Frame, state: &AppState, area: Rect) {
    let file = match state.current_file() {
        Some(f) => f,
        None => return,
    };

    let half_width = area.width / 2;
    let mut all_lines: Vec<Line> = Vec::new();

    for (hi, hunk) in file.hunks.iter().enumerate() {
        let is_current = hi == state.hunk_index;

        // Hunk header spans both columns
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
        all_lines.push(Line::from(vec![
            marker,
            Span::styled(&hunk.header, header_style),
            Span::raw("  "),
            status_icon,
        ]));

        // Show comment below header if present
        if let Some(comment) = &hunk.comment {
            all_lines.push(Line::from(vec![
                Span::raw("    # "),
                Span::styled(comment, Style::default().fg(Color::Yellow)),
            ]));
        }

        // Expand current hunk in side-by-side
        if is_current {
            let mut old_line_num = hunk.old_start;
            let mut new_line_num = hunk.new_start;

            // Pair lines
            let mut removed_buf: Vec<&str> = Vec::new();
            let mut added_buf: Vec<&str> = Vec::new();
            let mut line_groups: Vec<SideBySideLine> = Vec::new();

            // Process hunk lines into groups
            for diff_line in &hunk.lines {
                match diff_line {
                    DiffLine::Context(s) => {
                        // Flush pending
                        flush_sbs_pairs(&mut line_groups, &mut removed_buf, &mut added_buf);
                        line_groups.push(SideBySideLine::Context(s.as_str()));
                    }
                    DiffLine::Removed(s) => {
                        removed_buf.push(s.as_str());
                    }
                    DiffLine::Added(s) => {
                        added_buf.push(s.as_str());
                    }
                    DiffLine::NoNewline => {}
                }
            }
            flush_sbs_pairs(&mut line_groups, &mut removed_buf, &mut added_buf);

            // Render each paired line
            for sbs_line in &line_groups {
                match sbs_line {
                    SideBySideLine::Context(s) => {
                        let left = format!("{:>4} │ {}", old_line_num, s);
                        let right = format!("{:>4} │ {}", new_line_num, s);
                        old_line_num += 1;
                        new_line_num += 1;

                        let left_truncated = truncate_str(&left, half_width as usize);
                        let right_truncated = truncate_str(&right, half_width as usize);

                        all_lines.push(Line::from(vec![
                            Span::styled(
                                format!("{:<w$}", left_truncated, w = half_width as usize),
                                Style::default().fg(Color::DarkGray),
                            ),
                            Span::styled(
                                format!("{:<w$}", right_truncated, w = half_width as usize),
                                Style::default().fg(Color::DarkGray),
                            ),
                        ]));
                    }
                    SideBySideLine::Changed(left_opt, right_opt) => {
                        let left_str = if let Some(s) = left_opt {
                            let num = format!("{:>4} │-{}", old_line_num, s);
                            old_line_num += 1;
                            num
                        } else {
                            "     │".to_string()
                        };
                        let right_str = if let Some(s) = right_opt {
                            let num = format!("{:>4} │+{}", new_line_num, s);
                            new_line_num += 1;
                            num
                        } else {
                            "     │".to_string()
                        };

                        let left_truncated = truncate_str(&left_str, half_width as usize);
                        let right_truncated = truncate_str(&right_str, half_width as usize);

                        all_lines.push(Line::from(vec![
                            Span::styled(
                                format!("{:<w$}", left_truncated, w = half_width as usize),
                                Style::default().fg(Color::Red),
                            ),
                            Span::styled(
                                format!("{:<w$}", right_truncated, w = half_width as usize),
                                Style::default().fg(Color::Green),
                            ),
                        ]));
                    }
                }
            }
        }
    }

    // Apply viewport
    let start = state.viewport_offset.min(all_lines.len());
    let end = (start + area.height as usize).min(all_lines.len());
    let visible: Vec<Line> = all_lines[start..end].to_vec();

    let paragraph = Paragraph::new(visible).block(Block::default().borders(Borders::NONE));
    frame.render_widget(paragraph, area);
}

/// Diff view with viewport scrolling
fn render_diff_view(frame: &mut Frame, state: &AppState, area: Rect) {
    // Fall back to unified if terminal too narrow
    let use_side_by_side = state.diff_view_mode == DiffViewMode::SideBySide && area.width >= 100;

    if use_side_by_side {
        render_side_by_side(frame, state, area);
    } else {
        let all_lines = build_virtual_doc(state);

        // Slice to viewport
        let start = state.viewport_offset.min(all_lines.len());
        let end = (start + area.height as usize).min(all_lines.len());
        let visible: Vec<Line> = all_lines[start..end].to_vec();

        let paragraph = Paragraph::new(visible).block(Block::default().borders(Borders::NONE));
        frame.render_widget(paragraph, area);
    }
}

/// Status bar
fn render_status_bar(frame: &mut Frame, state: &AppState, area: Rect) {
    let text = match state.mode {
        AppMode::ConfirmQuit => " Quit? Unsaved review will be lost. (y/n)".to_string(),
        AppMode::Search => {
            format!(
                " /{}\u{2588}                              (Enter: search, Esc: cancel)",
                state.search_query
            )
        }
        AppMode::CommentEdit => {
            format!(
                " comment: {}\u{2588}                    (Enter: save, Esc: cancel)",
                state.comment_input
            )
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
            let search_hint = if state.has_active_search() {
                let idx = state.search_index.map_or(0, |i| i + 1);
                format!(" | [{}/{}] n/N:match", idx, state.search_matches.len())
            } else {
                String::new()
            };
            format!(
                " file {}/{} | hunk {}/{} | reviewed: {}/{} [a:{} r:{}]{} | j/k a/r ?:help q:quit",
                state.file_index + 1,
                state.diff.files.len(),
                current,
                total,
                reviewed,
                total,
                accepted,
                rejected,
                search_hint,
            )
        }
    };

    let paragraph =
        Paragraph::new(text).style(Style::default().bg(Color::DarkGray).fg(Color::White));
    frame.render_widget(paragraph, area);
}

/// Stats overlay
fn render_stats_overlay(frame: &mut Frame, state: &AppState) {
    let area = centered_rect(60, 70, frame.area());

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(
        "Diff Summary",
        Style::default().fg(Color::Yellow),
    )));
    lines.push(Line::from(""));

    // File list
    for (i, file) in state.diff.files.iter().enumerate() {
        let is_cursor = i == state.stats_cursor;
        let marker = if is_cursor { ">" } else { " " };

        let added = file.lines_added();
        let removed = file.lines_removed();

        let review_icon = match file.review_summary() {
            FileReviewSummary::AllAccepted => Span::styled(" ✓", Style::default().fg(Color::Green)),
            FileReviewSummary::HasRejected => Span::styled(" ✗", Style::default().fg(Color::Red)),
            FileReviewSummary::Partial => Span::styled(" ~", Style::default().fg(Color::Yellow)),
            FileReviewSummary::AllPending | FileReviewSummary::Empty => Span::raw("  "),
        };

        let bg = if is_cursor {
            Some(Color::DarkGray)
        } else {
            None
        };

        let marker_style = Style::default().fg(Color::Yellow);
        let marker_style = if let Some(bg_color) = bg {
            marker_style.bg(bg_color)
        } else {
            marker_style
        };

        let path_style = Style::default().fg(Color::White);
        let path_style = if let Some(bg_color) = bg {
            path_style.bg(bg_color)
        } else {
            path_style
        };

        let add_style = Style::default().fg(Color::Green);
        let add_style = if let Some(bg_color) = bg {
            add_style.bg(bg_color)
        } else {
            add_style
        };

        let rem_style = Style::default().fg(Color::Red);
        let rem_style = if let Some(bg_color) = bg {
            rem_style.bg(bg_color)
        } else {
            rem_style
        };

        lines.push(Line::from(vec![
            Span::styled(format!("{} ", marker), marker_style),
            Span::styled(&file.new_path, path_style),
            Span::styled(format!("  +{}", added), add_style),
            Span::styled(format!(" -{}", removed), rem_style),
            review_icon,
        ]));
    }

    lines.push(Line::from(""));

    // Totals
    let total_files = state.diff.files.len();
    let total_added: usize = state.diff.files.iter().map(|f| f.lines_added()).sum();
    let total_removed: usize = state.diff.files.iter().map(|f| f.lines_removed()).sum();
    let total_hunks = state.total_hunks();
    let reviewed = state.reviewed_hunks();
    let accepted = state.accepted_hunks();
    let rejected = reviewed - accepted;

    lines.push(Line::from(vec![
        Span::styled(" Total: ", Style::default().fg(Color::White)),
        Span::styled(
            format!("{} files  ", total_files),
            Style::default().fg(Color::White),
        ),
        Span::styled(
            format!("+{}", total_added),
            Style::default().fg(Color::Green),
        ),
        Span::styled(
            format!(" -{}", total_removed),
            Style::default().fg(Color::Red),
        ),
    ]));

    lines.push(Line::from(vec![
        Span::styled(" Reviewed: ", Style::default().fg(Color::White)),
        Span::styled(
            format!(
                "{}/{} hunks [a:{} r:{}]",
                reviewed, total_hunks, accepted, rejected
            ),
            Style::default().fg(Color::White),
        ),
    ]));

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " j/k:navigate Enter:go s/Esc:close",
        Style::default().fg(Color::DarkGray),
    )));

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Diff Summary ")
        .style(Style::default().bg(Color::Black));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Help overlay
fn render_help_overlay(frame: &mut Frame, _state: &AppState) {
    let area = centered_rect(60, 70, frame.area());

    let help_text = vec![
        Line::from(Span::styled(
            "Key Bindings",
            Style::default().fg(Color::Yellow),
        )),
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
        Line::from(vec![
            Span::styled("c         ", Style::default().fg(Color::Cyan)),
            Span::raw("Add/edit comment on hunk"),
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
            Span::styled("d         ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle side-by-side view"),
        ]),
        Line::from(vec![
            Span::styled("f         ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle file tree"),
        ]),
        Line::from(vec![
            Span::styled("h         ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle syntax highlighting"),
        ]),
        Line::from(vec![
            Span::styled("m         ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle mouse mode"),
        ]),
        Line::from(vec![
            Span::styled("s         ", Style::default().fg(Color::Cyan)),
            Span::raw("Diff summary"),
        ]),
        Line::from(vec![
            Span::styled("/         ", Style::default().fg(Color::Cyan)),
            Span::raw("Search in diff"),
        ]),
        Line::from(vec![
            Span::styled("n/N       ", Style::default().fg(Color::Cyan)),
            Span::raw("Next/prev match (or file)"),
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
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
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
        old_start: u32,
        old_count: u32,
        new_start: u32,
        new_count: u32,
        lines: Vec<DiffLine>,
    ) -> Hunk {
        Hunk {
            header: format!(
                "@@ -{},{} +{},{} @@",
                old_start, old_count, new_start, new_count
            ),
            old_start,
            old_count,
            new_start,
            new_count,
            lines,
            status: ReviewStatus::Pending,
            comment: None,
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
        let state = make_state_for_render(vec![make_hunk_with_lines(
            10,
            3,
            10,
            3,
            vec![
                DiffLine::Context("line1".to_string()),
                DiffLine::Context("line2".to_string()),
                DiffLine::Context("line3".to_string()),
            ],
        )]);
        let lines = build_virtual_doc(&state);
        // line 0 = header, lines 1-3 = context lines
        assert_eq!(lines.len(), 4);
        // Check that line numbers appear in the spans
        let line1_text: String = lines[1].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            line1_text.contains("10"),
            "Should contain old line 10: {}",
            line1_text
        );
    }

    #[test]
    fn test_line_numbers_added() {
        let state = make_state_for_render(vec![make_hunk_with_lines(
            1,
            2,
            1,
            3,
            vec![
                DiffLine::Context("ctx".to_string()),
                DiffLine::Added("new".to_string()),
                DiffLine::Context("ctx2".to_string()),
            ],
        )]);
        let lines = build_virtual_doc(&state);
        // header + 3 lines = 4
        assert_eq!(lines.len(), 4);
        // Added line (index 2) should have '+' marker
        let added_text: String = lines[2].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            added_text.contains("+new"),
            "Should contain +new: {}",
            added_text
        );
    }

    #[test]
    fn test_line_numbers_removed() {
        let state = make_state_for_render(vec![make_hunk_with_lines(
            5,
            3,
            5,
            2,
            vec![
                DiffLine::Context("ctx".to_string()),
                DiffLine::Removed("old".to_string()),
                DiffLine::Context("ctx2".to_string()),
            ],
        )]);
        let lines = build_virtual_doc(&state);
        assert_eq!(lines.len(), 4);
        // Removed line (index 2) should have '-' marker
        let removed_text: String = lines[2].spans.iter().map(|s| s.content.as_ref()).collect();
        assert!(
            removed_text.contains("-old"),
            "Should contain -old: {}",
            removed_text
        );
    }

    #[test]
    fn test_side_by_side_pair_lines() {
        // Test the line pairing logic
        let mut removed_buf: Vec<&str> = vec!["old1", "old2"];
        let mut added_buf: Vec<&str> = vec!["new1", "new2", "new3"];
        let mut groups: Vec<SideBySideLine> = Vec::new();

        flush_sbs_pairs(&mut groups, &mut removed_buf, &mut added_buf);

        assert_eq!(groups.len(), 3);
        assert!(removed_buf.is_empty());
        assert!(added_buf.is_empty());
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello world!", 8), "hello...");
        assert_eq!(truncate_str("hi", 2), "hi");
    }
}
