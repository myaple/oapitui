use crate::app::App;
use crate::views::request_builder::{FocusedPane, RowKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let rb = &app.request_builder;

    let title = format!(" {} {} ", rb.method, rb.path_template);
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, super::title_style()));
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
            let req_marker = if row.required { "*" } else { " " };
            let value_display = if is_selected && editing_params {
                format!("{}_", row.value)
            } else {
                row.value.clone()
            };
            let style = if is_selected {
                super::selected_style()
            } else {
                Style::default()
            };
            Row::new(vec![
                Cell::from(Span::styled(req_marker, Style::default().fg(Color::Red))),
                Cell::from(Span::styled(kind_label, Style::default().fg(Color::DarkGray))),
                Cell::from(Span::styled(row.name.clone(), Style::default().fg(Color::Cyan))),
                Cell::from(Span::styled(
                    row.type_label.clone(),
                    Style::default().fg(Color::DarkGray),
                )),
                Cell::from(value_display),
            ])
            .style(style)
        })
        .collect();

    let header = Row::new(vec!["", "kind", "name", "type", "value"])
        .style(
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Yellow),
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
            Style::default().fg(Color::Yellow)
        } else {
            Style::default()
        });

    let table = Table::new(param_rows, widths)
        .header(header)
        .block(params_block)
        .row_highlight_style(super::selected_style());

    f.render_stateful_widget(table, chunks[0], &mut table_state);

    // ── Body editor (bottom pane) ─────────────────────────────────────────────

    if has_body && chunks.len() > 1 {
        if let Some(body_row) = rb.rows.iter().find(|r| r.kind == RowKind::Body) {
            let (border_style, title_hint) = match rb.focus {
                FocusedPane::ParamsNav | FocusedPane::ParamsEdit => (
                    Style::default(),
                    format!(" Body ({}) — Tab=focus ", body_row.type_label),
                ),
                FocusedPane::BodyNormal => (
                    Style::default().fg(Color::Yellow),
                    format!(
                        " Body ({}) — NORMAL  hjkl=move  0/$=line  i/a=insert  Tab/Esc=params ",
                        body_row.type_label
                    ),
                ),
                FocusedPane::BodyInsert => (
                    Style::default().fg(Color::Green),
                    format!(" Body ({}) — INSERT  Esc=normal ", body_row.type_label),
                ),
            };

            let body_block = Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .title(title_hint);

            let value_with_cursor = match rb.focus {
                FocusedPane::BodyNormal | FocusedPane::BodyInsert => {
                    let cursor_char = if rb.focus == FocusedPane::BodyInsert { '│' } else { '█' };
                    let mut s = body_row.value.clone();
                    let byte_idx = s
                        .char_indices()
                        .nth(rb.cursor)
                        .map(|(i, _)| i)
                        .unwrap_or(s.len());
                    s.insert(byte_idx, cursor_char);
                    s
                }
                _ => body_row.value.clone(),
            };

            let body_para = Paragraph::new(value_with_cursor)
                .block(body_block)
                .wrap(ratatui::widgets::Wrap { trim: false });
            f.render_widget(body_para, chunks[1]);
        }
    }

    // ── Empty state hint ──────────────────────────────────────────────────────

    if rb.rows.is_empty() {
        let hint = Paragraph::new("No parameters. Press Enter to send.")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(hint, inner);
    }

    // ── Method badge (top-right overlay) ──────────────────────────────────────

    let method_area = Rect::new(
        area.x + area.width.saturating_sub(12),
        area.y,
        10,
        1,
    );
    let badge = Paragraph::new(rb.method.as_str()).style(
        Style::default()
            .fg(super::method_color(&rb.method))
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(badge, method_area);
}
