use crate::app::App;
use crate::theme::Theme;
use crate::views::request_builder::{FocusedPane, RowKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let rb = &app.request_builder;

    let title = format!(" {} {} ", rb.method, rb.path_template);
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, super::title_style(&app.theme)));
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let has_body = rb.has_body();

    let chunks = if has_body {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(inner)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)])
            .split(inner)
    };

    // ── Params table (top pane) ───────────────────────────────────────────────

    let params_focused = matches!(rb.focus, FocusedPane::ParamsNav | FocusedPane::ParamsEdit);
    let editing_params = rb.focus == FocusedPane::ParamsEdit;

    // Collect only non-body rows; their indices within `rows` are contiguous
    // from 0 because body is always appended last.
    let param_rows: Vec<Row> = rb
        .rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.kind != RowKind::Body)
        .map(|(i, row)| {
            let is_selected = params_focused && i == rb.selected;
            let kind_label = match row.kind {
                RowKind::PathParam => "path",
                RowKind::QueryParam => "query",
                RowKind::Header => "header",
                RowKind::Body => "body",
            };
            // Required params always show * in red.
            // Optional enabled params show + in green (opted in).
            // Optional disabled params show - in gray (skipped on send).
            let (req_marker, marker_color) = if row.required {
                ("*", app.theme.param_required)
            } else if row.enabled {
                ("+", app.theme.indicator_success)
            } else {
                ("-", app.theme.text_secondary)
            };

            let value_display = if is_selected && editing_params {
                format!("{}_", row.value)
            } else {
                row.value.clone()
            };

            let disabled = !row.required && !row.enabled;
            let row_style = if is_selected {
                super::selected_style(&app.theme)
            } else if disabled {
                Style::default().add_modifier(Modifier::DIM)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(Span::styled(req_marker, Style::default().fg(marker_color))),
                Cell::from(Span::styled(
                    kind_label,
                    Style::default().fg(app.theme.text_secondary),
                )),
                Cell::from(Span::styled(
                    row.name.clone(),
                    Style::default().fg(app.theme.text_key),
                )),
                Cell::from(Span::styled(
                    row.type_label.clone(),
                    Style::default().fg(app.theme.text_secondary),
                )),
                Cell::from(value_display),
            ])
            .style(row_style)
        })
        .collect();

    let header = Row::new(vec!["", "kind", "name", "type", "value"])
        .style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(app.theme.text_accent),
        )
        .height(1);

    let widths = [
        Constraint::Length(1),
        Constraint::Length(7),
        Constraint::Percentage(25),
        Constraint::Length(10),
        Constraint::Min(20),
    ];

    let mut table_state = TableState::default();
    if params_focused {
        table_state.select(Some(rb.selected));
    }

    let params_block = Block::default()
        .borders(Borders::BOTTOM)
        .border_style(if params_focused {
            Style::default().fg(app.theme.border_active)
        } else {
            Style::default()
        });

    let table = Table::new(param_rows, widths)
        .header(header)
        .block(params_block)
        .row_highlight_style(super::selected_style(&app.theme));

    f.render_stateful_widget(table, chunks[0], &mut table_state);

    // ── Body editor (bottom pane) ─────────────────────────────────────────────

    if has_body && chunks.len() > 1 {
        if let Some(body_row) = rb.rows.iter().find(|r| r.kind == RowKind::Body) {
            let ct_hint = if rb.body_content_type_count() > 1 {
                format!(
                    " [{}/{}] t=cycle",
                    rb.body_alt_index + 1,
                    rb.body_content_type_count()
                )
            } else {
                String::new()
            };
            let (border_style, title_hint) = match rb.focus {
                FocusedPane::ParamsNav | FocusedPane::ParamsEdit => (
                    Style::default(),
                    format!(" Body ({}){ct_hint} — Tab=focus ", body_row.type_label),
                ),
                FocusedPane::BodyNormal => (
                    Style::default().fg(app.theme.border_active),
                    format!(
                        " Body ({}){ct_hint} — NORMAL  hjkl=move  0/$=line  i/a=insert  Tab/Esc=params ",
                        body_row.type_label
                    ),
                ),
                FocusedPane::BodyInsert => (
                    Style::default().fg(app.theme.border_editing),
                    format!(" Body ({}){ct_hint} — INSERT  Esc=normal ", body_row.type_label),
                ),
            };

            let body_block = Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title_hint);

            let body_content: Text = match rb.focus {
                FocusedPane::BodyNormal => {
                    body_with_block_cursor(&body_row.value, rb.cursor, &app.theme)
                }
                FocusedPane::BodyInsert => {
                    body_with_bar_cursor(&body_row.value, rb.cursor, &app.theme)
                }
                _ => Text::raw(&body_row.value),
            };

            // Compute which line the cursor is on, then derive a scroll offset
            // that keeps the cursor inside the visible area.
            let cursor_line = {
                let chars: Vec<char> = body_row.value.chars().collect();
                chars[..rb.cursor.min(chars.len())]
                    .iter()
                    .filter(|&&c| c == '\n')
                    .count() as u16
            };
            let visible_lines = chunks[1].height.saturating_sub(2); // subtract borders
            let scroll_top = if cursor_line >= visible_lines {
                cursor_line - visible_lines + 1
            } else {
                0
            };

            let body_para = Paragraph::new(body_content)
                .block(body_block)
                .scroll((scroll_top, 0))
                .wrap(ratatui::widgets::Wrap { trim: false });
            f.render_widget(body_para, chunks[1]);
        }
    }

    // ── Empty state hint ──────────────────────────────────────────────────────

    if rb.rows.is_empty() {
        let hint = Paragraph::new("No parameters. Press Enter to send.")
            .style(Style::default().fg(app.theme.text_secondary));
        f.render_widget(hint, inner);
    }

    // ── Method badge (top-right overlay) ──────────────────────────────────────

    let method_area = Rect::new(area.x + area.width.saturating_sub(12), area.y, 10, 1);
    let badge = Paragraph::new(rb.method.as_str()).style(
        Style::default()
            .fg(super::method_color(&rb.method, &app.theme))
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(badge, method_area);

    // ── Curl popup ────────────────────────────────────────────────────────────

    if rb.show_curl {
        let curl = rb.curl_command();
        let lines: Vec<Line> = curl.lines().map(|l| Line::from(l.to_string())).collect();
        let line_count = lines.len() as u16;
        let popup_height = (line_count + 2).min(area.height.saturating_sub(4));
        let popup_area = super::centered_rect_fixed(70, popup_height, area);

        f.render_widget(Clear, popup_area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.border_focused))
            .title(Span::styled(
                " curl command — current values  y=copy  Esc/c=close ",
                super::title_style(&app.theme),
            ));
        let para = Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: false });
        f.render_widget(para, popup_area);
    }
}

/// Render body text with the character at `cursor` highlighted (block cursor,
/// normal mode). No extra character is inserted into the string.
fn body_with_block_cursor(value: &str, cursor: usize, theme: &Theme) -> Text<'static> {
    let block_style = Style::default()
        .fg(theme.cursor_block_fg)
        .bg(theme.cursor_block_bg);
    let lines: Vec<Line> = value
        .split('\n')
        .scan(0usize, |pos, line_str| {
            let line_chars: Vec<char> = line_str.chars().collect();
            let line_start = *pos;
            let line_end = line_start + line_chars.len();
            // Advance past this line + its newline
            *pos = line_end + 1;

            let rendered = if cursor >= line_start && cursor <= line_end {
                let col = cursor - line_start;
                let before: String = line_chars[..col].iter().collect();
                let (at, after): (String, String) = if col < line_chars.len() {
                    (
                        line_chars[col].to_string(),
                        line_chars[col + 1..].iter().collect(),
                    )
                } else {
                    // Cursor is past end of line — show a space block
                    (" ".to_string(), String::new())
                };
                Line::from(vec![
                    Span::raw(before),
                    Span::styled(at, block_style),
                    Span::raw(after),
                ])
            } else {
                Line::raw(line_str.to_owned())
            };
            Some(rendered)
        })
        .collect();
    Text::from(lines)
}

/// Render body text with a `│` bar inserted at `cursor` (insert mode).
fn body_with_bar_cursor(value: &str, cursor: usize, theme: &Theme) -> Text<'static> {
    let bar_style = Style::default().fg(theme.cursor_bar);
    let lines: Vec<Line> = value
        .split('\n')
        .scan(0usize, |pos, line_str| {
            let line_chars: Vec<char> = line_str.chars().collect();
            let line_start = *pos;
            let line_end = line_start + line_chars.len();
            *pos = line_end + 1;

            let rendered = if cursor >= line_start && cursor <= line_end {
                let col = cursor - line_start;
                let before: String = line_chars[..col].iter().collect();
                let after: String = line_chars[col..].iter().collect();
                Line::from(vec![
                    Span::raw(before),
                    Span::styled("│", bar_style),
                    Span::raw(after),
                ])
            } else {
                Line::raw(line_str.to_owned())
            };
            Some(rendered)
        })
        .collect();
    Text::from(lines)
}
