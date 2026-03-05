use crate::app::App;
use crate::views::add_server::AddServerField;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let popup = super::centered_rect(70, 80, area);
    f.render_widget(Clear, popup);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" Add Server ", super::title_style(&app.theme)));

    let inner = block.inner(popup);
    f.render_widget(block, popup);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Name
            Constraint::Length(1),
            Constraint::Length(3), // URL
            Constraint::Length(1),
            Constraint::Length(3), // Client Cert
            Constraint::Length(1),
            Constraint::Length(3), // Client Key
            Constraint::Length(1),
            Constraint::Length(3), // CA Cert
            Constraint::Min(0),
        ])
        .split(inner);

    let active_style = Style::default()
        .fg(app.theme.border_active)
        .add_modifier(Modifier::BOLD);
    let inactive_style = Style::default().fg(app.theme.border_unfocused);

    let field_style = |f: AddServerField| {
        if app.add_server.field == f {
            active_style
        } else {
            inactive_style
        }
    };

    let name_input = Paragraph::new(app.add_server.name.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(field_style(AddServerField::Name))
            .title(" Name "),
    );
    f.render_widget(name_input, rows[0]);

    let url_input = Paragraph::new(app.add_server.url.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(field_style(AddServerField::Url))
            .title(" OpenAPI URL or file path "),
    );
    f.render_widget(url_input, rows[2]);

    let cert_input = Paragraph::new(app.add_server.client_cert.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(field_style(AddServerField::ClientCert))
            .title(" Client Cert (mTLS, optional) "),
    );
    f.render_widget(cert_input, rows[4]);

    let key_input = Paragraph::new(app.add_server.client_key.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(field_style(AddServerField::ClientKey))
            .title(" Client Key (mTLS, optional) "),
    );
    f.render_widget(key_input, rows[6]);

    let ca_input = Paragraph::new(app.add_server.ca_cert.as_str()).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(field_style(AddServerField::CaCert))
            .title(" CA Cert (optional) "),
    );
    f.render_widget(ca_input, rows[8]);

    // Place terminal cursor in the active field
    let (cur_row, cur_val) = match app.add_server.field {
        AddServerField::Name => (rows[0], &app.add_server.name),
        AddServerField::Url => (rows[2], &app.add_server.url),
        AddServerField::ClientCert => (rows[4], &app.add_server.client_cert),
        AddServerField::ClientKey => (rows[6], &app.add_server.client_key),
        AddServerField::CaCert => (rows[8], &app.add_server.ca_cert),
    };
    f.set_cursor_position((cur_row.x + cur_val.len() as u16 + 1, cur_row.y + 1));
}
