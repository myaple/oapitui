use crate::app::App;
use oapitui_openapi::Endpoint;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
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

    // Horizontal split: list on left, detail panel on right
    let h_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(inner);

    let list_area = h_chunks[0];
    let detail_area = h_chunks[1];

    // ── Left pane: endpoint list ─────────────────────────────────────────────
    let show_filter_bar = el.filter_active || !el.filter.is_empty();
    let list_chunks = if show_filter_bar {
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
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD)
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

    let list_border_style = if el.detail_focused {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };
    let list_block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(list_border_style);

    let list = List::new(items)
        .highlight_style(super::selected_style())
        .block(list_block);
    f.render_stateful_widget(list, list_chunks[0], &mut list_state);

    if show_filter_bar {
        let (text, style) = if el.filter_active {
            (
                format!("/ {}_", el.filter),
                Style::default().fg(Color::Yellow),
            )
        } else {
            (
                format!("/ {} (Enter to open, Esc to clear)", el.filter),
                Style::default().fg(Color::Cyan),
            )
        };
        let filter_bar = Paragraph::new(text).style(style);
        f.render_widget(filter_bar, list_chunks[1]);
    }

    // ── Right pane: detail sub-panel ─────────────────────────────────────────
    let detail_border_style = if el.detail_focused {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let detail_block = Block::default()
        .borders(Borders::LEFT)
        .border_style(detail_border_style)
        .title(Span::styled(
            " Detail (Tab to focus, j/k to scroll) ",
            if el.detail_focused {
                super::title_style()
            } else {
                Style::default().fg(Color::DarkGray)
            },
        ));

    let detail_inner = detail_block.inner(detail_area);
    f.render_widget(detail_block, detail_area);

    if let Some(ep) = filtered.get(el.selected) {
        let lines = build_detail_lines(ep);
        let total_lines = lines.len() as u16;

        let para = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((el.detail_scroll, 0));
        f.render_widget(para, detail_inner);

        // Scrollbar on the right edge of the detail inner area
        if total_lines > detail_inner.height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
            let mut scrollbar_state = ScrollbarState::new(
                total_lines.saturating_sub(detail_inner.height) as usize,
            )
            .position(el.detail_scroll as usize);
            f.render_stateful_widget(scrollbar, detail_inner, &mut scrollbar_state);
        }
    } else {
        let placeholder = Paragraph::new(Span::styled(
            "Select an endpoint to view details",
            Style::default().fg(Color::DarkGray),
        ));
        f.render_widget(placeholder, detail_inner);
    }
}

fn build_detail_lines(ep: &Endpoint) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    // ── Method + Path header ─────────────────────────────────────────────────
    let method_color = super::method_color(&ep.method);
    lines.push(Line::from(vec![
        Span::styled(
            ep.method.clone(),
            Style::default()
                .fg(method_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            ep.path.clone(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    // ── Summary ──────────────────────────────────────────────────────────────
    if !ep.summary.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Summary  ", Style::default().fg(Color::DarkGray)),
            Span::styled(ep.summary.clone(), Style::default().fg(Color::White)),
        ]));
    }

    // ── Description (if distinct from summary) ────────────────────────────────
    if !ep.description.is_empty() && ep.description != ep.summary {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Description",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));
        for desc_line in ep.description.lines() {
            lines.push(Line::from(Span::styled(
                desc_line.to_string(),
                Style::default().fg(Color::White),
            )));
        }
    }

    // ── Operation ID ─────────────────────────────────────────────────────────
    if let Some(op_id) = &ep.operation_id {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Operation ", Style::default().fg(Color::DarkGray)),
            Span::styled(op_id.clone(), Style::default().fg(Color::Yellow)),
        ]));
    }

    // ── Tags ─────────────────────────────────────────────────────────────────
    if !ep.tags.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Tags     ", Style::default().fg(Color::DarkGray)),
            Span::styled(ep.tags.join(", "), Style::default().fg(Color::Magenta)),
        ]));
    }

    // ── Parameters ───────────────────────────────────────────────────────────
    if !ep.parameters.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_divider("Parameters"));

        for param in &ep.parameters {
            lines.push(Line::from(""));
            let req_style = if param.required {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::DarkGray)
            };
            let req_marker = if param.required { "* " } else { "  " };
            lines.push(Line::from(vec![
                Span::styled(req_marker, req_style),
                Span::styled(
                    param.name.clone(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!("[{}]", param.location),
                    Style::default().fg(Color::Blue),
                ),
                Span::styled("  ", Style::default()),
                Span::styled(
                    param.schema_type.clone(),
                    Style::default().fg(Color::Green),
                ),
            ]));

            if !param.description.is_empty() {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        param.description.clone(),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }

            let example_str = serde_json::to_string(&param.example).unwrap_or_default();
            if !example_str.is_empty() && example_str != "\"\"" && example_str != "null" {
                lines.push(Line::from(vec![
                    Span::styled("  example  ", Style::default().fg(Color::DarkGray)),
                    Span::styled(example_str, Style::default().fg(Color::Cyan)),
                ]));
            }
        }
    }

    // ── Request Body ─────────────────────────────────────────────────────────
    if let Some(body) = &ep.request_body {
        lines.push(Line::from(""));
        lines.push(section_divider("Request Body"));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Content-Type  ", Style::default().fg(Color::DarkGray)),
            Span::styled(body.content_type.clone(), Style::default().fg(Color::Yellow)),
        ]));

        if !body.description.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Description   ", Style::default().fg(Color::DarkGray)),
                Span::styled(body.description.clone(), Style::default().fg(Color::White)),
            ]));
        }

        let example_str =
            serde_json::to_string_pretty(&body.example).unwrap_or_else(|_| "{}".to_string());
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Schema example:",
            Style::default().fg(Color::DarkGray),
        )));
        for ex_line in example_str.lines() {
            lines.push(Line::from(Span::styled(
                ex_line.to_string(),
                Style::default().fg(Color::Green),
            )));
        }
    }

    // ── Responses ────────────────────────────────────────────────────────────
    if !ep.responses.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_divider("Responses"));
        lines.push(Line::from(""));
        let response_spans: Vec<Span> = ep
            .responses
            .iter()
            .enumerate()
            .flat_map(|(i, code)| {
                let status: u16 = code.parse().unwrap_or(0);
                let color = super::status_color(status);
                let mut spans = vec![Span::styled(
                    code.clone(),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                )];
                if i + 1 < ep.responses.len() {
                    spans.push(Span::styled("  ", Style::default()));
                }
                spans
            })
            .collect();
        lines.push(Line::from(response_spans));
    }

    lines.push(Line::from(""));
    lines
}

fn section_divider(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!("── {} ", title),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ))
}
