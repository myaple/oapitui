use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let rv = &app.response_viewer;

    let status_color = super::status_color(rv.status);
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
                    Span::styled(format!("{k}: "), Style::default().fg(Color::Cyan)),
                    Span::raw(v),
                ])
            })
            .collect();

        let headers_widget = Paragraph::new(header_lines)
            .block(Block::default().borders(Borders::BOTTOM).title(" Headers "));
        f.render_widget(headers_widget, chunks[0]);

        render_body(f, rv, chunks[1]);
    } else {
        render_body(f, rv, inner);
    }
}

fn render_body(f: &mut Frame, rv: &crate::views::response_viewer::ResponseViewerState, area: Rect) {
    // Simple JSON syntax coloring: lines starting with keys get cyan keys
    let lines: Vec<Line> = rv.body.lines().map(colorize_json_line).collect();

    let para = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((rv.scroll, 0))
        .block(Block::default().borders(Borders::TOP).title(" Body "));
    f.render_widget(para, area);
}

fn colorize_json_line(line: &str) -> Line<'static> {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    let indent_str = " ".repeat(indent);

    // Check if line has key: value pattern
    if let Some(colon) = trimmed.find(':') {
        let key_part = &trimmed[..colon + 1];
        let val_part = &trimmed[colon + 1..];
        let val_trimmed = val_part.trim_start();

        let val_color = if val_trimmed.starts_with('"') {
            Color::Green
        } else if val_trimmed.starts_with(|c: char| c.is_ascii_digit() || c == '-') {
            Color::Yellow
        } else if val_trimmed == "true" || val_trimmed == "false" {
            Color::Magenta
        } else if val_trimmed == "null" {
            Color::DarkGray
        } else {
            Color::White
        };

        Line::from(vec![
            Span::raw(indent_str),
            Span::styled(key_part.to_string(), Style::default().fg(Color::Cyan)),
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
