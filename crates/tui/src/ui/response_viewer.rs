use crate::app::App;
use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let rv = &app.response_viewer;

    let status_color = app.theme.status_color(rv.status);
    let title = format!(
        " Response — {} {} — {}ms ",
        rv.status,
        status_text(rv.status),
        rv.elapsed_ms
    );

    let outer = Block::default().borders(Borders::ALL).title(Span::styled(
        title,
        Style::default()
            .fg(status_color)
            .add_modifier(Modifier::BOLD),
    ));

    let inner = outer.inner(area);
    f.render_widget(outer, area);

    if rv.show_headers {
        // Split: headers top, body bottom
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(inner);

        let header_lines: Vec<Line> = rv
            .headers
            .iter()
            .map(|(k, v)| {
                Line::from(vec![
                    Span::styled(format!("{k}: "), Style::default().fg(app.theme.text_key)),
                    Span::raw(v),
                ])
            })
            .collect();

        let headers_widget = Paragraph::new(header_lines)
            .block(Block::default().borders(Borders::BOTTOM).title(" Headers "));
        f.render_widget(headers_widget, chunks[0]);

        render_body(f, rv, chunks[1], &app.theme);
        rv.page_size.set(chunks[1].height);
    } else {
        render_body(f, rv, inner, &app.theme);
        rv.page_size.set(inner.height);
    }

    // ── Save-to-file dialog ───────────────────────────────────────────────────
    if let Some(ref filename) = rv.save_dialog {
        let popup_area = super::centered_rect_fixed(50, 5, area);
        f.render_widget(Clear, popup_area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.border_focused))
            .title(Span::styled(
                " Save response body to file ",
                super::title_style(&app.theme),
            ));
        let text = format!("Filename: {filename}_");
        let para = Paragraph::new(text)
            .block(block)
            .style(Style::default().fg(app.theme.text_primary));
        f.render_widget(para, popup_area);
    }
}

fn render_body(
    f: &mut Frame,
    rv: &crate::views::response_viewer::ResponseViewerState,
    area: Rect,
    theme: &Theme,
) {
    // Simple JSON syntax coloring: lines starting with keys get cyan keys
    let lines: Vec<Line> = rv
        .body
        .lines()
        .map(|line| colorize_json_line(line, theme))
        .collect();

    let para = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((rv.scroll, 0))
        .block(Block::default().borders(Borders::TOP).title(" Body "));
    f.render_widget(para, area);
}

fn colorize_json_line(line: &str, theme: &Theme) -> Line<'static> {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    let indent_str = " ".repeat(indent);

    // Check if line has key: value pattern
    if let Some(colon) = trimmed.find(':') {
        let key_part = &trimmed[..colon + 1];
        let val_part = &trimmed[colon + 1..];
        let val_trimmed = val_part.trim_start();

        let val_color = if val_trimmed.starts_with('"') {
            theme.json_string
        } else if val_trimmed.starts_with(|c: char| c.is_ascii_digit() || c == '-') {
            theme.json_number
        } else if val_trimmed == "true" || val_trimmed == "false" {
            theme.json_bool
        } else if val_trimmed == "null" {
            theme.json_null
        } else {
            theme.text_primary
        };

        Line::from(vec![
            Span::raw(indent_str),
            Span::styled(key_part.to_string(), Style::default().fg(theme.text_key)),
            Span::raw(" "),
            Span::styled(val_trimmed.to_string(), Style::default().fg(val_color)),
        ])
    } else {
        Line::from(Span::raw(line.to_string()))
    }
}

fn status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "",
    }
}
