use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let el = &app.endpoint_list;

    let title = format!(" {} — Endpoints ", el.server_name);
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, super::title_style()));

    let inner = outer.inner(area);
    f.render_widget(outer, area);

    // Filter bar at bottom of inner area if active
    let chunks = if el.filter_active {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(inner)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)])
            .split(inner)
    };

    let list_area = chunks[0];
    let filtered = el.filtered();

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, ep)| {
            let method_span = Span::styled(
                format!(" {:7}", ep.method),
                Style::default()
                    .fg(super::method_color(&ep.method))
                    .add_modifier(Modifier::BOLD),
            );
            let path_span = Span::styled(
                format!(" {}", ep.path),
                if i == el.selected {
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            );
            let summary_span = if !ep.summary.is_empty() {
                Span::styled(
                    format!("  {}", ep.summary),
                    Style::default().fg(Color::DarkGray),
                )
            } else {
                Span::raw("")
            };

            ListItem::new(Line::from(vec![method_span, path_span, summary_span]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(el.selected));

    let list = List::new(items).highlight_style(super::selected_style());
    f.render_stateful_widget(list, list_area, &mut list_state);

    if el.filter_active {
        let filter_bar = Paragraph::new(format!("/ {}", el.filter))
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(filter_bar, chunks[1]);
    }

    // Detail panel to the right if an endpoint is selected
    if let Some(ep) = filtered.get(el.selected) {
        // We actually show everything inline in list — could split view later
        let _ = ep;
    }
}
