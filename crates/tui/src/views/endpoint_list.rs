use oapitui_openapi::Endpoint;
use std::cell::Cell;

#[derive(Default, Clone, Copy, PartialEq)]
pub enum SortMode {
    #[default]
    None,
    ByMethod,
    ByPath,
}

impl SortMode {
    pub fn next(self) -> Self {
        match self {
            Self::None => Self::ByMethod,
            Self::ByMethod => Self::ByPath,
            Self::ByPath => Self::None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ByMethod => "method",
            Self::ByPath => "path",
        }
    }
}

pub struct EndpointListState {
    pub endpoints: Vec<Endpoint>,
    pub server_name: String,
    pub server_base: String,
    pub selected: usize,
    pub filter: String,
    pub filter_active: bool,
    pub detail_scroll: u16,
    pub detail_focused: bool,
    pub sort_mode: SortMode,
    pub show_curl: bool,
    /// Height of the visible list area — updated by the UI renderer each frame.
    pub page_size: Cell<u16>,
}

impl Default for EndpointListState {
    fn default() -> Self {
        Self {
            endpoints: Vec::new(),
            server_name: String::new(),
            server_base: String::new(),
            selected: 0,
            filter: String::new(),
            filter_active: false,
            detail_scroll: 0,
            detail_focused: false,
            sort_mode: SortMode::None,
            show_curl: false,
            page_size: Cell::new(10),
        }
    }
}

impl EndpointListState {
    pub fn new(endpoints: Vec<Endpoint>, server_name: String, server_base: String) -> Self {
        Self {
            endpoints,
            server_name,
            server_base,
            ..Default::default()
        }
    }

    pub fn filtered(&self) -> Vec<&Endpoint> {
        let q = self.filter.to_lowercase();
        let mut list: Vec<&Endpoint> = self
            .endpoints
            .iter()
            .filter(|e| {
                q.is_empty()
                    || e.path.to_lowercase().contains(&q)
                    || e.method.to_lowercase().contains(&q)
                    || e.summary.to_lowercase().contains(&q)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect();

        match self.sort_mode {
            SortMode::None => {}
            SortMode::ByMethod => list.sort_by(|a, b| a.method.cmp(&b.method)),
            SortMode::ByPath => list.sort_by(|a, b| a.path.cmp(&b.path)),
        }

        list
    }

    pub fn selected_endpoint(&self) -> Option<&Endpoint> {
        let list = self.filtered();
        list.get(self.selected).copied()
    }

    pub fn next(&mut self) {
        let n = self.filtered().len();
        if n > 0 {
            self.selected = (self.selected + 1).min(n - 1);
            self.detail_scroll = 0;
        }
    }

    pub fn prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.detail_scroll = 0;
        }
    }

    pub fn home(&mut self) {
        self.selected = 0;
        self.detail_scroll = 0;
    }

    pub fn end(&mut self) {
        let n = self.filtered().len();
        if n > 0 {
            self.selected = n - 1;
            self.detail_scroll = 0;
        }
    }

    pub fn page_up(&mut self) {
        let page = self.page_size.get().max(1) as usize;
        self.selected = self.selected.saturating_sub(page);
        self.detail_scroll = 0;
    }

    pub fn page_down(&mut self) {
        let n = self.filtered().len();
        let page = self.page_size.get().max(1) as usize;
        if n > 0 {
            self.selected = (self.selected + page).min(n - 1);
            self.detail_scroll = 0;
        }
    }
}
