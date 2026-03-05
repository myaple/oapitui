use crate::app::App;
use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};

pub fn render(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let picker = match &app.env_picker {
        Some(p) => p,
        None => return,
    };

    let height = (picker.count as u16 + 2).min(area.height.saturating_sub(4));
    let popup_area = super::centered_rect_fixed(40, height, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.border_focused))
        .title(Span::styled(
            " Select Environment (Enter/Esc) ",
            super::title_style(&app.theme),
        ));

    let mut items: Vec<ListItem> = vec![ListItem::new(Line::from(Span::styled(
        "  (none)",
        Style::default().fg(app.theme.text_secondary),
    )))];

    for env in &app.config.environments {
        let var_count = env.variables.len();
        items.push(ListItem::new(Line::from(vec![
            Span::styled(
                format!("  {}", env.name),
                Style::default().fg(app.theme.text_primary),
            ),
            Span::styled(
                format!("  ({var_count} vars)"),
                Style::default().fg(app.theme.text_secondary),
            ),
        ])));
    }

    let mut state = ListState::default();
    state.select(Some(picker.selected));

    let list = List::new(items)
        .block(block)
        .highlight_style(super::selected_style(&app.theme));
    f.render_stateful_widget(list, popup_area, &mut state);
}
