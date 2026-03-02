use crate::app::App;
use crate::views::add_server::AddServerField;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let popup = super::centered_rect(60, 30, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" Add Server ", super::title_style()));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(inner);

    let active_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let inactive_style = Style::default().fg(Color::DarkGray);

    let name_style = if app.add_server.field == AddServerField::Name {
        active_style
    } else {
        inactive_style
    };
    let url_style = if app.add_server.field == AddServerField::Url {
        active_style
    } else {
        inactive_style
    };

    let name_input = Paragraph::new(app.add_server.name.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(name_style)
                .title(" Name "),
        );
    f.render_widget(name_input, rows[0]);

    let url_input = Paragraph::new(app.add_server.url.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(url_style)
                .title(" OpenAPI JSON URL "),
        );
    f.render_widget(url_input, rows[2]);

    // Show cursor in active field
    let (cur_row, cur_val) = match app.add_server.field {
        AddServerField::Name => (rows[0], &app.add_server.name),
        AddServerField::Url => (rows[2], &app.add_server.url),
    };
    f.set_cursor_position((
        cur_row.x + cur_val.len() as u16 + 1,
        cur_row.y + 1,
    ));
}
