use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(" oapitui — Servers ", super::title_style()));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if app.config.servers.is_empty() {
        let msg = Paragraph::new("No servers configured. Press 'a' to add one.")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(msg, inner);
        return;
    }

    // Split: list on left, detail panel on right
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(inner);

    // --- Server list ---
    let items: Vec<ListItem> = app
        .config
        .servers
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let loading = app.spec_loading.contains_key(&s.name);
            let loaded = app.specs.contains_key(&s.name);

            let status_icon = if loading {
                Span::styled("⟳ ", Style::default().fg(Color::Yellow))
            } else if loaded {
                Span::styled("✓ ", Style::default().fg(Color::Green))
            } else {
                Span::styled("✗ ", Style::default().fg(Color::Red))
            };

            let name = Span::styled(
                s.name.clone(),
                if i == app.server_list.selected {
                    super::selected_style()
                } else {
                    Style::default()
                },
            );

            ListItem::new(Line::from(vec![status_icon, name]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(app.server_list.selected));

    let list = List::new(items)
        .highlight_style(super::selected_style())
        .block(Block::default().borders(Borders::RIGHT));

    f.render_stateful_widget(list, chunks[0], &mut list_state);

    // --- Detail panel ---
    if let Some(server) = app.config.servers.get(app.server_list.selected) {
        // "URL:  " prefix is 6 chars; truncate so the URL never wraps mid-string.
        let url_budget = chunks[1].width.saturating_sub(6) as usize;
        let url_chars: Vec<char> = server.url.chars().collect();
        let url_display = if url_chars.len() > url_budget {
            format!("{}…", url_chars[..url_budget.saturating_sub(1)].iter().collect::<String>())
        } else {
            server.url.clone()
        };

        let mut lines: Vec<Line> = vec![
            Line::from(vec![
                Span::styled("Name: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&server.name),
            ]),
            Line::from(vec![
                Span::styled("URL:  ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(url_display, Style::default().fg(Color::Blue)),
            ]),
        ];

        if !server.description.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Desc: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&server.description),
            ]));
        }

        if let Some(spec) = app.specs.get(&server.name) {
            let ep_count = oaitui_openapi::extract_endpoints(spec).len();
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled("Endpoints: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(ep_count.to_string(), Style::default().fg(Color::Cyan)),
            ]));
            if let Some(refreshed) = app.last_refreshed.get(&server.name) {
                lines.push(Line::from(vec![
                    Span::styled("Refreshed: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(
                        format_elapsed(refreshed.elapsed().as_secs()),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
            if let Some(info_desc) = &spec.info.description {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    info_desc.clone(),
                    Style::default().fg(Color::DarkGray),
                )));
            }
        } else if app.spec_loading.contains_key(&server.name) {
            lines.push(Line::from(Span::styled(
                "Loading spec…",
                Style::default().fg(Color::Yellow),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                "Spec not loaded — press 'r' to fetch",
                Style::default().fg(Color::Red),
            )));
        }

        let detail = Paragraph::new(lines).wrap(Wrap { trim: false });
        f.render_widget(detail, chunks[1]);
    }
}

fn format_elapsed(secs: u64) -> String {
    if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else if secs < 86400 {
        format!("{}h ago", secs / 3600)
    } else {
        format!("{}d ago", secs / 86400)
    }
}
