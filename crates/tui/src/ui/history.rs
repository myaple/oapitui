use crate::app::App;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let hs = &app.history;

    let title = format!(" History ({} entries) ", hs.entries.len());
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, super::title_style(&app.theme)));

    let inner = outer.inner(area);
    f.render_widget(outer, area);

    if hs.entries.is_empty() {
        let msg = Paragraph::new("No history yet. Send a request to see it here.")
            .style(Style::default().fg(app.theme.text_secondary));
        f.render_widget(msg, inner);
        return;
    }

    // Horizontal split: list on left, detail on right
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner);

    let list_area = chunks[0];
    let detail_area = chunks[1];

    // ── Left pane: history list ──────────────────────────────────────────────
    let show_filter = hs.filter_active || !hs.filter.is_empty();
    let list_chunks = if show_filter {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(1)])
            .split(list_area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0)])
            .split(list_area)
    };

    hs.page_size.set(list_chunks[0].height);

    let filtered = hs.filtered();
    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let method_span = Span::styled(
                format!(" {:7}", entry.method),
                Style::default()
                    .fg(super::method_color(&entry.method, &app.theme))
                    .add_modifier(Modifier::BOLD),
            );
            let status_color = app.theme.status_color(entry.status);
            let status_span = Span::styled(
                format!(" {} ", entry.status),
                Style::default().fg(status_color),
            );
            let path_span = Span::styled(
                format!(" {}", entry.path),
                if i == hs.selected {
                    Style::default()
                        .fg(app.theme.text_primary)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(app.theme.text_primary)
                },
            );
            let time_span = Span::styled(
                format!("  {}ms", entry.elapsed_ms),
                Style::default().fg(app.theme.text_secondary),
            );

            ListItem::new(Line::from(vec![
                method_span,
                status_span,
                path_span,
                time_span,
            ]))
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(hs.selected));

    let list_block = Block::default().borders(Borders::RIGHT);
    let list = List::new(items)
        .highlight_style(super::selected_style(&app.theme))
        .block(list_block);
    f.render_stateful_widget(list, list_chunks[0], &mut list_state);

    if show_filter {
        let (text, style) = if hs.filter_active {
            (
                format!("/ {}_", hs.filter),
                Style::default().fg(app.theme.filter_active),
            )
        } else {
            (
                format!("/ {} (Esc to clear)", hs.filter),
                Style::default().fg(app.theme.filter_inactive),
            )
        };
        f.render_widget(Paragraph::new(text).style(style), list_chunks[1]);
    }

    // ── Right pane: detail ───────────────────────────────────────────────────
    let detail_block = Block::default()
        .borders(Borders::LEFT)
        .title(Span::styled(
            " Detail ",
            Style::default().fg(app.theme.text_secondary),
        ));
    let detail_inner = detail_block.inner(detail_area);
    f.render_widget(detail_block, detail_area);

    if let Some(entry) = filtered.get(hs.selected) {
        let lines = build_detail(entry, app);
        let para = Paragraph::new(lines).wrap(Wrap { trim: false });
        f.render_widget(para, detail_inner);
    }
}

fn build_detail(entry: &oapitui_config::HistoryEntry, app: &App) -> Vec<Line<'static>> {
    let bold = Style::default().add_modifier(Modifier::BOLD);
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                entry.method.clone(),
                Style::default()
                    .fg(app.theme.method_color(&entry.method))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(
                entry.path.clone(),
                Style::default()
                    .fg(app.theme.text_primary)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Server:    ", bold),
            Span::raw(entry.server_name.clone()),
        ]),
        Line::from(vec![
            Span::styled("URL:       ", bold),
            Span::styled(
                entry.url.clone(),
                Style::default().fg(app.theme.text_url),
            ),
        ]),
        Line::from(vec![
            Span::styled("Status:    ", bold),
            Span::styled(
                entry.status.to_string(),
                Style::default().fg(app.theme.status_color(entry.status)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Elapsed:   ", bold),
            Span::raw(format!("{}ms", entry.elapsed_ms)),
        ]),
        Line::from(vec![
            Span::styled("Timestamp: ", bold),
            Span::styled(
                entry.timestamp.clone(),
                Style::default().fg(app.theme.text_secondary),
            ),
        ]),
    ];

    if !entry.params.is_empty() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "── Parameters ",
            Style::default()
                .fg(app.theme.title)
                .add_modifier(Modifier::BOLD),
        )));
        let mut params: Vec<_> = entry.params.iter().collect();
        params.sort_by_key(|(k, _)| k.as_str());
        for (k, v) in params {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {k}: "),
                    Style::default().fg(app.theme.text_key),
                ),
                Span::raw(v.clone()),
            ]));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Enter = re-open endpoint  d = delete  Esc = back",
        Style::default().fg(app.theme.text_secondary),
    )));

    lines
}
