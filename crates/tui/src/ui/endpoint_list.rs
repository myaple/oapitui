use crate::app::{build_curl_command, App};
use crate::theme::Theme;
use oapitui_openapi::Endpoint;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: Rect) {
    let el = &app.endpoint_list;

    let sort_label = el.sort_mode.label();
    let title = if sort_label == "none" {
        format!(" {} — Endpoints ", el.server_name)
    } else {
        format!(" {} — Endpoints  [sort: {}] ", el.server_name, sort_label)
    };
    let outer = Block::default()
        .borders(Borders::ALL)
        .title(Span::styled(title, super::title_style(&app.theme)));

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

    // Update page_size so keyboard handler can page correctly
    el.page_size.set(list_chunks[0].height);

    let filtered = el.filtered();

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(i, ep)| {
            let method_span = Span::styled(
                format!(" {:7}", ep.method),
                Style::default()
                    .fg(super::method_color(&ep.method, &app.theme))
                    .add_modifier(Modifier::BOLD),
            );
            let path_span = Span::styled(
                format!(" {}", ep.path),
                if i == el.selected {
                    Style::default()
                        .fg(app.theme.text_primary)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(app.theme.text_primary)
                },
            );
            let summary_span = if !ep.summary.is_empty() {
                Span::styled(
                    format!("  {}", ep.summary),
                    Style::default().fg(app.theme.text_secondary),
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
        Style::default().fg(app.theme.border_unfocused)
    } else {
        Style::default().fg(app.theme.text_primary)
    };
    let list_block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(list_border_style);

    let list = List::new(items)
        .highlight_style(super::selected_style(&app.theme))
        .block(list_block);
    f.render_stateful_widget(list, list_chunks[0], &mut list_state);

    if show_filter_bar {
        let (text, style) = if el.filter_active {
            (
                format!("/ {}_", el.filter),
                Style::default().fg(app.theme.filter_active),
            )
        } else {
            (
                format!("/ {} (Enter to open, Esc to clear)", el.filter),
                Style::default().fg(app.theme.filter_inactive),
            )
        };
        let filter_bar = Paragraph::new(text).style(style);
        f.render_widget(filter_bar, list_chunks[1]);
    }

    // ── Right pane: detail sub-panel ─────────────────────────────────────────
    let detail_border_style = if el.detail_focused {
        Style::default().fg(app.theme.border_focused)
    } else {
        Style::default().fg(app.theme.border_unfocused)
    };
    let detail_block = Block::default()
        .borders(Borders::LEFT)
        .border_style(detail_border_style)
        .title(Span::styled(
            " Detail (Tab to focus, j/k to scroll) ",
            if el.detail_focused {
                super::title_style(&app.theme)
            } else {
                Style::default().fg(app.theme.text_secondary)
            },
        ));

    let detail_inner = detail_block.inner(detail_area);
    f.render_widget(detail_block, detail_area);

    if let Some(ep) = filtered.get(el.selected) {
        let lines = build_detail_lines(ep, &app.theme);
        let total_lines = lines.len() as u16;

        let para = Paragraph::new(lines)
            .wrap(Wrap { trim: false })
            .scroll((el.detail_scroll, 0));
        f.render_widget(para, detail_inner);

        // Scrollbar on the right edge of the detail inner area
        if total_lines > detail_inner.height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
            let mut scrollbar_state =
                ScrollbarState::new(total_lines.saturating_sub(detail_inner.height) as usize)
                    .position(el.detail_scroll as usize);
            f.render_stateful_widget(scrollbar, detail_inner, &mut scrollbar_state);
        }
    } else {
        let placeholder = Paragraph::new(Span::styled(
            "Select an endpoint to view details",
            Style::default().fg(app.theme.text_secondary),
        ));
        f.render_widget(placeholder, detail_inner);
    }

    // ── Curl popup ────────────────────────────────────────────────────────────
    if el.show_curl {
        if let Some(ep) = filtered.get(el.selected) {
            let curl = build_curl_command(&el.server_base, ep);
            let lines: Vec<Line> = curl.lines().map(|l| Line::from(l.to_string())).collect();
            let line_count = lines.len() as u16;
            let popup_height = (line_count + 2).min(area.height.saturating_sub(4));
            let popup_area = super::centered_rect_fixed(70, popup_height, area);

            f.render_widget(Clear, popup_area);
            let block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(app.theme.border_focused))
                .title(Span::styled(
                    " curl command (Esc/c to close) ",
                    super::title_style(&app.theme),
                ));
            let para = Paragraph::new(lines)
                .block(block)
                .wrap(Wrap { trim: false });
            f.render_widget(para, popup_area);
        }
    }
}

fn build_detail_lines(ep: &Endpoint, theme: &Theme) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    // ── Method + Path header ─────────────────────────────────────────────────
    let method_color = theme.method_color(&ep.method);
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
                .fg(theme.text_primary)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    lines.push(Line::from(""));

    // ── Summary ──────────────────────────────────────────────────────────────
    if !ep.summary.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Summary  ", Style::default().fg(theme.text_secondary)),
            Span::styled(ep.summary.clone(), Style::default().fg(theme.text_primary)),
        ]));
    }

    // ── Description (if distinct from summary) ────────────────────────────────
    if !ep.description.is_empty() && ep.description != ep.summary {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Description",
            Style::default()
                .fg(theme.title)
                .add_modifier(Modifier::BOLD),
        )));
        for desc_line in ep.description.lines() {
            lines.push(Line::from(Span::styled(
                desc_line.to_string(),
                Style::default().fg(theme.text_primary),
            )));
        }
    }

    // ── Operation ID ─────────────────────────────────────────────────────────
    if let Some(op_id) = &ep.operation_id {
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Operation ", Style::default().fg(theme.text_secondary)),
            Span::styled(op_id.clone(), Style::default().fg(theme.text_accent)),
        ]));
    }

    // ── Tags ─────────────────────────────────────────────────────────────────
    if !ep.tags.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Tags     ", Style::default().fg(theme.text_secondary)),
            Span::styled(ep.tags.join(", "), Style::default().fg(theme.text_tag)),
        ]));
    }

    // ── Parameters ───────────────────────────────────────────────────────────
    if !ep.parameters.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_divider("Parameters", theme));

        for param in &ep.parameters {
            lines.push(Line::from(""));
            let req_style = if param.required {
                Style::default().fg(theme.param_required)
            } else {
                Style::default().fg(theme.text_secondary)
            };
            let req_marker = if param.required { "* " } else { "  " };
            lines.push(Line::from(vec![
                Span::styled(req_marker, req_style),
                Span::styled(
                    param.name.clone(),
                    Style::default()
                        .fg(theme.text_primary)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!("[{}]", param.location),
                    Style::default().fg(theme.param_location),
                ),
                Span::styled("  ", Style::default()),
                Span::styled(
                    param.schema_type.clone(),
                    Style::default().fg(theme.param_type),
                ),
            ]));

            if !param.description.is_empty() {
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(
                        param.description.clone(),
                        Style::default().fg(theme.text_secondary),
                    ),
                ]));
            }

            let example_str = serde_json::to_string(&param.example).unwrap_or_default();
            if !example_str.is_empty() && example_str != "\"\"" && example_str != "null" {
                lines.push(Line::from(vec![
                    Span::styled("  example  ", Style::default().fg(theme.text_secondary)),
                    Span::styled(example_str, Style::default().fg(theme.param_example)),
                ]));
            }
        }
    }

    // ── Request Body ─────────────────────────────────────────────────────────
    if let Some(body) = &ep.request_body {
        lines.push(Line::from(""));
        lines.push(section_divider("Request Body", theme));
        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("Content-Type  ", Style::default().fg(theme.text_secondary)),
            Span::styled(
                body.content_type.clone(),
                Style::default().fg(theme.text_accent),
            ),
        ]));

        if !body.description.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("Description   ", Style::default().fg(theme.text_secondary)),
                Span::styled(
                    body.description.clone(),
                    Style::default().fg(theme.text_primary),
                ),
            ]));
        }

        let example_str =
            serde_json::to_string_pretty(&body.example).unwrap_or_else(|_| "{}".to_string());
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Schema example:",
            Style::default().fg(theme.text_secondary),
        )));
        for ex_line in example_str.lines() {
            lines.push(Line::from(Span::styled(
                ex_line.to_string(),
                Style::default().fg(theme.md_code),
            )));
        }
    }

    // ── Responses ────────────────────────────────────────────────────────────
    if !ep.responses.is_empty() {
        lines.push(Line::from(""));
        lines.push(section_divider("Responses", theme));
        lines.push(Line::from(""));
        let response_spans: Vec<Span> = ep
            .responses
            .iter()
            .enumerate()
            .flat_map(|(i, code)| {
                let status: u16 = code.parse().unwrap_or(0);
                let color = theme.status_color(status);
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

fn section_divider(title: &str, theme: &Theme) -> Line<'static> {
    Line::from(Span::styled(
        format!("── {} ", title),
        Style::default()
            .fg(theme.title)
            .add_modifier(Modifier::BOLD),
    ))
}
