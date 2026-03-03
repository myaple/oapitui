mod add_server;
mod endpoint_list;
mod request_builder;
mod response_viewer;
mod server_list;

use crate::app::{App, Screen};
use crate::theme::Theme;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    // Split off bottom help bar
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    match app.screen {
        Screen::ServerList => server_list::render(f, app, chunks[0]),
        Screen::AddServer => {
            server_list::render(f, app, chunks[0]);
            add_server::render(f, app, chunks[0]);
        }
        Screen::EndpointList => endpoint_list::render(f, app, chunks[0]),
        Screen::RequestBuilder => request_builder::render(f, app, chunks[0]),
        Screen::ResponseViewer => response_viewer::render(f, app, chunks[0]),
    }

    render_help_bar(f, app, chunks[1]);
    render_error(f, app, area);
}

fn render_help_bar(f: &mut Frame, app: &App, area: Rect) {
    let keys: &[(&str, &str)] = match app.screen {
        Screen::ServerList => &[
            ("↑/k↓/j", "navigate"),
            ("PgUp/PgDn", "page"),
            ("Home/End", "first/last"),
            ("Enter", "open"),
            ("a", "add"),
            ("d", "delete"),
            ("r", "refresh"),
            ("q/^C", "quit"),
        ],
        Screen::AddServer => &[
            ("Tab", "switch field"),
            ("Enter", "confirm"),
            ("Esc", "cancel"),
        ],
        Screen::EndpointList => &[
            ("↑/k↓/j", "navigate"),
            ("PgUp/PgDn", "page"),
            ("Home/End", "first/last"),
            ("/", "filter"),
            ("s", "sort"),
            ("c", "curl"),
            ("Tab", "detail"),
            ("Enter", "open"),
            ("q", "quit"),
        ],
        Screen::RequestBuilder => &[
            ("↑/k↓/j", "navigate rows"),
            ("Space", "toggle optional"),
            ("e", "edit"),
            ("Esc", "stop edit / back"),
            ("Enter", "send"),
            ("q", "quit"),
        ],
        Screen::ResponseViewer => &[
            ("↑/k↓/j", "scroll"),
            ("PgUp/PgDn", "page"),
            ("Home/End", "top/bottom"),
            ("h", "toggle headers"),
            ("s", "save"),
            ("q", "quit"),
        ],
    };

    let spans: Vec<Span> = keys
        .iter()
        .flat_map(|(k, v)| {
            vec![
                Span::styled(format!(" {k}"), Style::default().fg(app.theme.help_key)),
                Span::styled(format!(" {v} "), Style::default().fg(app.theme.help_desc)),
            ]
        })
        .collect();

    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_error(f: &mut Frame, app: &App, area: Rect) {
    if let Some(err) = &app.error {
        let width = area.width.saturating_sub(4);
        // Calculate how many lines the text needs when wrapped inside the block
        // (inner width = block width minus 2 border chars)
        let inner_width = width.saturating_sub(2) as usize;
        let text_lines = if inner_width == 0 {
            1
        } else {
            err.lines()
                .map(|line| line.len().max(1).div_ceil(inner_width))
                .sum::<usize>()
                .max(1)
        };
        let height = (text_lines as u16 + 2).min(8); // +2 for borders, cap at 8 rows
        let x = 2;
        let y = area.height.saturating_sub(height + 2);
        let popup_area = Rect::new(x, y, width, height);

        f.render_widget(Clear, popup_area);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(app.theme.error))
            .title(" Error — press any key to dismiss ");
        let msg = Paragraph::new(err.as_str())
            .style(Style::default().fg(app.theme.error))
            .wrap(Wrap { trim: true })
            .block(block);
        f.render_widget(msg, popup_area);
    }
}

/// Centered rectangle with fixed dimensions (in columns / rows).
pub fn centered_rect_fixed(width: u16, height: u16, r: Rect) -> Rect {
    let width = width.min(r.width);
    let height = height.min(r.height);
    let x = r.x + (r.width.saturating_sub(width)) / 2;
    let y = r.y + (r.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

/// Centered rectangle helper.
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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

/// Colour for HTTP method badge.
pub fn method_color(method: &str, theme: &Theme) -> Color {
    theme.method_color(method)
}

pub fn title_style(theme: &Theme) -> Style {
    Style::default()
        .fg(theme.title)
        .add_modifier(Modifier::BOLD)
}

pub fn selected_style(theme: &Theme) -> Style {
    Style::default()
        .bg(theme.selected_bg)
        .add_modifier(Modifier::BOLD)
}
