use oapitui_config::HistoryEntry;
use std::cell::Cell;

pub struct HistoryState {
    pub entries: Vec<HistoryEntry>,
    pub selected: usize,
    /// Height of the visible list area — updated by the UI renderer each frame.
    pub page_size: Cell<u16>,
    pub filter: String,
    pub filter_active: bool,
}

impl Default for HistoryState {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            selected: 0,
            page_size: Cell::new(10),
            filter: String::new(),
            filter_active: false,
        }
    }
}

impl HistoryState {
    pub fn new(entries: Vec<HistoryEntry>) -> Self {
        Self {
            entries,
            ..Default::default()
        }
    }

    pub fn filtered(&self) -> Vec<&HistoryEntry> {
        let q = self.filter.to_lowercase();
        self.entries
            .iter()
            .rev() // most recent first
            .filter(|e| {
                q.is_empty()
                    || e.path.to_lowercase().contains(&q)
                    || e.method.to_lowercase().contains(&q)
                    || e.server_name.to_lowercase().contains(&q)
            })
            .collect()
    }

    pub fn selected_entry(&self) -> Option<&HistoryEntry> {
        self.filtered().get(self.selected).copied()
    }

    pub fn next(&mut self) {
        let n = self.filtered().len();
        if n > 0 {
            self.selected = (self.selected + 1).min(n - 1);
        }
    }

    pub fn prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn page_up(&mut self) {
        let page = self.page_size.get().max(1) as usize;
        self.selected = self.selected.saturating_sub(page);
    }

    pub fn page_down(&mut self) {
        let n = self.filtered().len();
        let page = self.page_size.get().max(1) as usize;
        if n > 0 {
            self.selected = (self.selected + page).min(n - 1);
        }
    }
}
