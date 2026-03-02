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
            format!(
                "{}…",
                url_chars[..url_budget.saturating_sub(1)]
                    .iter()
                    .collect::<String>()
            )
        } else {
            server.url.clone()
        };

        let bold = Style::default().add_modifier(Modifier::BOLD);
        let mut header: Vec<Line> = vec![
            Line::from(vec![Span::styled("Name: ", bold), Span::raw(&server.name)]),
            Line::from(vec![
                Span::styled("URL:  ", bold),
                Span::styled(url_display, Style::default().fg(Color::Blue)),
            ]),
        ];

        let spec_desc = if let Some(spec) = app.specs.get(&server.name) {
            let ep_count = oaitui_openapi::extract_endpoints(spec).len();
            header.push(Line::from(""));
            header.push(Line::from(vec![
                Span::styled("Endpoints: ", bold),
                Span::styled(ep_count.to_string(), Style::default().fg(Color::Cyan)),
            ]));
            if let Some(refreshed) = app.last_refreshed.get(&server.name) {
                header.push(Line::from(vec![
                    Span::styled("Refreshed: ", bold),
                    Span::styled(
                        format_elapsed(refreshed.elapsed().as_secs()),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }
            spec.info.description.clone()
        } else if app.spec_loading.contains_key(&server.name) {
            header.push(Line::from(Span::styled(
                "Loading spec…",
                Style::default().fg(Color::Yellow),
            )));
            None
        } else {
            header.push(Line::from(Span::styled(
                "Spec not loaded — press 'r' to fetch",
                Style::default().fg(Color::Red),
            )));
            None
        };

        // Split the detail column: fixed header on top, markdown description below.
        let header_height = header.len() as u16;
        let detail_split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(header_height), Constraint::Min(0)])
            .split(chunks[1]);

        f.render_widget(Paragraph::new(header), detail_split[0]);

        if let Some(desc) = spec_desc {
            let md_lines = markdown_to_lines(&desc);
            f.render_widget(
                Paragraph::new(md_lines).wrap(Wrap { trim: false }),
                detail_split[1],
            );
        }
    }
}

fn markdown_to_lines(input: &str) -> Vec<Line<'static>> {
    use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

    let parser = Parser::new_ext(input, Options::ENABLE_STRIKETHROUGH);
    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut spans: Vec<Span<'static>> = Vec::new();

    // Style flags
    let mut bold = false;
    let mut italic = false;
    let mut heading: Option<HeadingLevel> = None;
    let mut in_code_block = false;
    // Stack of ordered-list counters; None = unordered
    let mut list_stack: Vec<Option<u64>> = Vec::new();

    let make_style = |bold: bool, italic: bool, code: bool, h: Option<HeadingLevel>| -> Style {
        let mut s = Style::default();
        if code {
            return s.fg(Color::Green);
        }
        if bold {
            s = s.add_modifier(Modifier::BOLD);
        }
        if italic {
            s = s.add_modifier(Modifier::ITALIC);
        }
        if let Some(level) = h {
            s = s.add_modifier(Modifier::BOLD).fg(match level {
                HeadingLevel::H1 => Color::Yellow,
                HeadingLevel::H2 => Color::Cyan,
                _ => Color::White,
            });
        }
        s
    };

    macro_rules! flush {
        () => {
            if !spans.is_empty() {
                lines.push(Line::from(std::mem::take(&mut spans)));
            }
        };
    }

    for event in parser {
        match event {
            // ── Block structure ──────────────────────────────────────────────
            Event::Start(Tag::Heading { level, .. }) => {
                flush!();
                heading = Some(level);
            }
            Event::End(TagEnd::Heading(_)) => {
                flush!();
                lines.push(Line::from(""));
                heading = None;
            }
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                flush!();
                lines.push(Line::from(""));
            }
            Event::Start(Tag::CodeBlock(_)) => {
                flush!();
                in_code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                flush!();
                lines.push(Line::from(""));
                in_code_block = false;
            }
            Event::Start(Tag::List(start)) => {
                list_stack.push(start);
            }
            Event::End(TagEnd::List(_)) => {
                list_stack.pop();
                if list_stack.is_empty() {
                    lines.push(Line::from(""));
                }
            }
            Event::Start(Tag::Item) => {
                flush!();
                let prefix = match list_stack.last() {
                    Some(Some(n)) => format!("{}. ", n),
                    _ => "• ".to_string(),
                };
                spans.push(Span::raw(prefix));
            }
            Event::End(TagEnd::Item) => {
                // Advance ordered counter
                if let Some(Some(n)) = list_stack.last_mut() {
                    *n += 1;
                }
                flush!();
            }
            Event::Start(Tag::BlockQuote(_)) => {
                spans.push(Span::styled("▌ ", Style::default().fg(Color::DarkGray)));
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                flush!();
                lines.push(Line::from(""));
            }
            // ── Inline formatting ────────────────────────────────────────────
            Event::Start(Tag::Strong) => bold = true,
            Event::End(TagEnd::Strong) => bold = false,
            Event::Start(Tag::Emphasis) => italic = true,
            Event::End(TagEnd::Emphasis) => italic = false,
            Event::Start(Tag::Link { .. }) | Event::End(TagEnd::Link) => {}
            // ── Leaf content ─────────────────────────────────────────────────
            Event::Text(text) => {
                let style = make_style(bold, italic, in_code_block, heading);
                spans.push(Span::styled(text.into_string(), style));
            }
            Event::Code(text) => {
                // Inline code span
                spans.push(Span::styled(
                    format!("`{}`", text),
                    Style::default().fg(Color::Green),
                ));
            }
            Event::SoftBreak => spans.push(Span::raw(" ")),
            Event::HardBreak => flush!(),
            Event::Rule => {
                flush!();
                lines.push(Line::styled(
                    "─".repeat(40),
                    Style::default().fg(Color::DarkGray),
                ));
                lines.push(Line::from(""));
            }
            _ => {}
        }
    }

    flush!();
    lines
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
