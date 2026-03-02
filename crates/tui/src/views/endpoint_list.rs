use oapitui_openapi::Endpoint;

#[derive(Default)]
pub struct EndpointListState {
    pub endpoints: Vec<Endpoint>,
    pub server_name: String,
    pub server_base: String,
    pub selected: usize,
    pub filter: String,
    pub filter_active: bool,
}

impl EndpointListState {
    pub fn new(endpoints: Vec<Endpoint>, server_name: String, server_base: String) -> Self {
        Self {
            endpoints,
            server_name,
            server_base,
            selected: 0,
            filter: String::new(),
            filter_active: false,
        }
    }

    pub fn filtered(&self) -> Vec<&Endpoint> {
        let q = self.filter.to_lowercase();
        self.endpoints
            .iter()
            .filter(|e| {
                q.is_empty()
                    || e.path.to_lowercase().contains(&q)
                    || e.method.to_lowercase().contains(&q)
                    || e.summary.to_lowercase().contains(&q)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(&q))
            })
            .collect()
    }

    pub fn selected_endpoint(&self) -> Option<&Endpoint> {
        let list = self.filtered();
        list.get(self.selected).copied()
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
}
