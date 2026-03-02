use crate::app::App;
use crate::views::request_builder::RowKind;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let rb = &app.request_builder;

    let title = format!(
        " {} {} ",
        rb.method, rb.path_template
    );

    let outer = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, super::title_style()));

    let inner = outer.inner(area);
    f.render_widget(outer, area);

    // Split: table top, body editor bottom (if there's a body row)
    let has_body = rb.rows.iter().any(|r| r.kind == RowKind::Body);
    let body_selected = rb
        .rows
        .get(rb.selected)
        .map(|r| r.kind == RowKind::Body)
        .unwrap_or(false);

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

    let table_area = chunks[0];

    // Build table rows (non-body params)
    let non_body: Vec<_> = rb
        .rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.kind != RowKind::Body)
        .collect();

    let table_rows: Vec<Row> = non_body
        .iter()
        .map(|(i, row)| {
            let is_selected = *i == rb.selected;
            let kind_label = match row.kind {
                RowKind::PathParam => "path",
                RowKind::QueryParam => "query",
                RowKind::Header => "header",
                RowKind::Body => "body",
            };
            let req_marker = if row.required { "*" } else { " " };
            let value_display = if is_selected && rb.editing {
                format!("{}_", row.value) // show cursor underscore
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
                Cell::from(Span::styled(
                    kind_label,
                    Style::default().fg(Color::DarkGray),
                )),
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
    // Map selected index to non-body list index
    let selected_in_table = non_body.iter().position(|(i, _)| *i == rb.selected);
    table_state.select(selected_in_table);

    let table = Table::new(table_rows, widths)
        .header(header)
        .block(Block::default().borders(Borders::BOTTOM))
        .row_highlight_style(super::selected_style());

    f.render_stateful_widget(table, table_area, &mut table_state);

    // Body editor panel
    if has_body && chunks.len() > 1 {
        if let Some(body_row) = rb.rows.iter().find(|r| r.kind == RowKind::Body) {
            let body_block = Block::default()
                .borders(Borders::ALL)
                .border_style(if body_selected {
                    Style::default().fg(Color::Yellow)
                } else {
                    Style::default()
                })
                .title(format!(
                    " Body ({}) — e=edit  Backspace=del  Esc=done  Enter=newline ",
                    body_row.type_label
                ));

            let value_with_cursor = if body_selected && rb.editing {
                // Show cursor at actual cursor position
                let mut s = body_row.value.clone();
                let byte_idx = s
                    .char_indices()
                    .nth(rb.cursor)
                    .map(|(i, _)| i)
                    .unwrap_or(s.len());
                s.insert(byte_idx, '█');
                s
            } else {
                body_row.value.clone()
            };

            let body_para = Paragraph::new(value_with_cursor)
                .block(body_block)
                .wrap(ratatui::widgets::Wrap { trim: false });
            f.render_widget(body_para, chunks[1]);
        }
    }

    // If no rows, show hint
    if rb.rows.is_empty() {
        let hint = Paragraph::new("No parameters. Press Enter to send.")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(hint, inner);
    }

    // Method badge overlay in top-right
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
