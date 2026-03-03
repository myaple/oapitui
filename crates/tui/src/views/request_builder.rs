use oapitui_client::RequestDef;
use oapitui_openapi::Endpoint;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum RowKind {
    PathParam,
    QueryParam,
    Header,
    Body,
}

/// Which pane has focus and what mode it is in.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum FocusedPane {
    /// Top pane: navigating param rows with j/k.
    #[default]
    ParamsNav,
    /// Top pane: editing a param value inline.
    ParamsEdit,
    /// Bottom pane: vim normal mode (cursor moves, no typing).
    BodyNormal,
    /// Bottom pane: insert mode (typing edits the body).
    BodyInsert,
}

#[derive(Debug, Clone)]
pub struct FieldRow {
    pub kind: RowKind,
    pub name: String,
    pub type_label: String,
    pub value: String,
    pub required: bool,
    /// Whether this param will be included in the request.
    /// Required params are always enabled; optional params start disabled so
    /// the user explicitly opts in to sending them.
    pub enabled: bool,
}

pub struct RequestBuilderState {
    pub method: String,
    pub base_url: String,
    pub path_template: String,
    pub rows: Vec<FieldRow>,
    /// Index within the *param* rows only (never points at the body row).
    pub selected: usize,
    pub focus: FocusedPane,
    /// Char-position cursor used when the body pane is focused.
    pub cursor: usize,
    /// First key of a pending two-key sequence (e.g. 'd' before 'dw'/'dd').
    pub pending_key: Option<char>,
}

impl Default for RequestBuilderState {
    fn default() -> Self {
        Self {
            method: "GET".to_string(),
            base_url: String::new(),
            path_template: String::new(),
            rows: vec![],
            selected: 0,
            focus: FocusedPane::ParamsNav,
            cursor: 0,
            pending_key: None,
        }
    }
}

impl RequestBuilderState {
    pub fn from_endpoint(
        ep: &Endpoint,
        base_url: String,
        default_headers: HashMap<String, String>,
    ) -> Self {
        let mut rows: Vec<FieldRow> = vec![];

        for p in &ep.parameters {
            let kind = match p.location.as_str() {
                "path" => RowKind::PathParam,
                "header" => RowKind::Header,
                _ => RowKind::QueryParam,
            };
            rows.push(FieldRow {
                kind,
                name: p.name.clone(),
                type_label: p.schema_type.clone(),
                value: value_to_string(&p.example),
                required: p.required,
                // Required params are always on; optional params start disabled
                // so the user opts in to sending them instead of opting out.
                enabled: p.required,
            });
        }

        for (k, v) in &default_headers {
            rows.push(FieldRow {
                kind: RowKind::Header,
                name: k.clone(),
                type_label: "string".to_string(),
                value: v.clone(),
                required: false,
                enabled: true, // user-configured defaults are on by default
            });
        }

        // Body is always appended last so param indices stay contiguous.
        if let Some(body) = &ep.request_body {
            let pretty = serde_json::to_string_pretty(&body.example).unwrap_or_default();
            rows.push(FieldRow {
                kind: RowKind::Body,
                name: "body".to_string(),
                type_label: body.content_type.clone(),
                value: pretty,
                required: false,
                enabled: true,
            });
        }

        Self {
            method: ep.method.clone(),
            base_url,
            path_template: ep.path.clone(),
            rows,
            selected: 0,
            focus: FocusedPane::ParamsNav,
            cursor: 0,
            pending_key: None,
        }
    }

    /// Number of non-body rows shown in the params table.
    pub fn param_count(&self) -> usize {
        self.rows.iter().filter(|r| r.kind != RowKind::Body).count()
    }

    pub fn has_body(&self) -> bool {
        self.rows.iter().any(|r| r.kind == RowKind::Body)
    }

    // ── Params-pane toggle ────────────────────────────────────────────────────

    /// Toggle the enabled state of the currently selected optional param.
    /// Required params cannot be toggled off.
    pub fn toggle_enabled(&mut self) {
        if let Some(row) = self.rows.get_mut(self.selected) {
            if !row.required {
                row.enabled = !row.enabled;
            }
        }
    }

    // ── Params-pane navigation ────────────────────────────────────────────────

    pub fn next_row(&mut self) {
        let n = self.param_count();
        if n > 0 {
            self.selected = (self.selected + 1).min(n - 1);
        }
    }

    pub fn prev_row(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    // ── Params-pane inline editing ────────────────────────────────────────────

    pub fn edit_char(&mut self, c: char) {
        if let Some(row) = self.rows.get_mut(self.selected) {
            let byte_idx = row
                .value
                .char_indices()
                .nth(self.cursor)
                .map(|(i, _)| i)
                .unwrap_or(row.value.len());
            row.value.insert(byte_idx, c);
            self.cursor += 1;
        }
    }

    pub fn edit_backspace(&mut self) {
        if let Some(row) = self.rows.get_mut(self.selected) {
            if self.cursor > 0 {
                let byte_idx = row
                    .value
                    .char_indices()
                    .nth(self.cursor - 1)
                    .map(|(i, _)| i)
                    .unwrap_or(row.value.len());
                row.value.remove(byte_idx);
                self.cursor -= 1;
            }
        }
    }

    // ── Body-pane editing ─────────────────────────────────────────────────────

    pub fn edit_body_char(&mut self, c: char) {
        if let Some(row) = self.rows.iter_mut().find(|r| r.kind == RowKind::Body) {
            let byte_idx = row
                .value
                .char_indices()
                .nth(self.cursor)
                .map(|(i, _)| i)
                .unwrap_or(row.value.len());
            row.value.insert(byte_idx, c);
            self.cursor += 1;
        }
    }

    pub fn edit_body_backspace(&mut self) {
        if let Some(row) = self.rows.iter_mut().find(|r| r.kind == RowKind::Body) {
            if self.cursor > 0 {
                let byte_idx = row
                    .value
                    .char_indices()
                    .nth(self.cursor - 1)
                    .map(|(i, _)| i)
                    .unwrap_or(row.value.len());
                row.value.remove(byte_idx);
                self.cursor -= 1;
            }
        }
    }

    // ── Body-pane vim cursor motions ──────────────────────────────────────────

    fn body_chars(&self) -> Vec<char> {
        self.rows
            .iter()
            .find(|r| r.kind == RowKind::Body)
            .map(|r| r.value.chars().collect())
            .unwrap_or_default()
    }

    pub fn cursor_left(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }

    pub fn cursor_right(&mut self) {
        let len = self.body_chars().len();
        if self.cursor < len {
            self.cursor += 1;
        }
    }

    pub fn cursor_up(&mut self) {
        let chars = self.body_chars();
        let line_start = chars[..self.cursor]
            .iter()
            .rposition(|&c| c == '\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        if line_start == 0 {
            return;
        }
        let col = self.cursor - line_start;
        let prev_line_end = line_start - 1;
        let prev_line_start = chars[..prev_line_end]
            .iter()
            .rposition(|&c| c == '\n')
            .map(|i| i + 1)
            .unwrap_or(0);
        let prev_line_len = prev_line_end - prev_line_start;
        self.cursor = prev_line_start + col.min(prev_line_len);
    }

    pub fn cursor_down(&mut self) {
        let chars = self.body_chars();
        let next_newline = chars[self.cursor..]
            .iter()
            .position(|&c| c == '\n')
            .map(|i| self.cursor + i);
        if let Some(nl) = next_newline {
            let line_start = chars[..self.cursor]
                .iter()
                .rposition(|&c| c == '\n')
                .map(|i| i + 1)
                .unwrap_or(0);
            let col = self.cursor - line_start;
            let next_start = nl + 1;
            let next_end = chars[next_start..]
                .iter()
                .position(|&c| c == '\n')
                .map(|i| next_start + i)
                .unwrap_or(chars.len());
            self.cursor = next_start + col.min(next_end - next_start);
        }
    }

    pub fn cursor_line_start(&mut self) {
        let chars = self.body_chars();
        self.cursor = chars[..self.cursor]
            .iter()
            .rposition(|&c| c == '\n')
            .map(|i| i + 1)
            .unwrap_or(0);
    }

    pub fn cursor_line_end(&mut self) {
        let chars = self.body_chars();
        self.cursor = chars[self.cursor..]
            .iter()
            .position(|&c| c == '\n')
            .map(|i| self.cursor + i)
            .unwrap_or(chars.len());
    }

    pub fn body_goto_top(&mut self) {
        self.cursor = 0;
    }

    pub fn body_goto_bottom(&mut self) {
        let chars = self.body_chars();
        self.cursor = chars
            .iter()
            .rposition(|&c| c == '\n')
            .map(|i| i + 1)
            .unwrap_or(0);
    }

    pub fn body_delete_line(&mut self) {
        if let Some(row) = self.rows.iter_mut().find(|r| r.kind == RowKind::Body) {
            let chars: Vec<char> = row.value.chars().collect();
            let line_start = chars[..self.cursor]
                .iter()
                .rposition(|&c| c == '\n')
                .map(|i| i + 1)
                .unwrap_or(0);
            // Position after the newline that ends this line, or end of string.
            let line_end = chars[line_start..]
                .iter()
                .position(|&c| c == '\n')
                .map(|i| line_start + i + 1)
                .unwrap_or(chars.len());
            // Last line: also consume the preceding newline so we don't leave
            // a trailing blank line.
            let (del_start, del_end) = if line_end == chars.len() && line_start > 0 {
                (line_start - 1, chars.len())
            } else {
                (line_start, line_end)
            };
            row.value = chars[..del_start]
                .iter()
                .chain(chars[del_end..].iter())
                .collect();
            self.cursor = del_start.min(row.value.chars().count());
        }
    }

    pub fn body_delete_word(&mut self) {
        if let Some(row) = self.rows.iter_mut().find(|r| r.kind == RowKind::Body) {
            let chars: Vec<char> = row.value.chars().collect();
            let start = self.cursor;
            if start >= chars.len() {
                return;
            }
            let mut end = start;
            if chars[end].is_alphanumeric() {
                // Eat alphanumeric run, then trailing spaces.
                while end < chars.len() && chars[end].is_alphanumeric() {
                    end += 1;
                }
                while end < chars.len() && chars[end] == ' ' {
                    end += 1;
                }
            } else if chars[end] == ' ' {
                // On whitespace: eat spaces (but not newlines).
                while end < chars.len() && chars[end] == ' ' {
                    end += 1;
                }
            }
            // On punctuation: no-op.
            if end > start {
                row.value = chars[..start].iter().chain(chars[end..].iter()).collect();
            }
        }
    }

    // ── Request assembly ──────────────────────────────────────────────────────

    pub fn build_request(&self) -> RequestDef {
        let mut path_params = HashMap::new();
        let mut query_params = HashMap::new();
        let mut headers = HashMap::new();
        let mut body: Option<Value> = None;

        for row in &self.rows {
            match row.kind {
                RowKind::PathParam => {
                    if row.enabled {
                        path_params.insert(row.name.clone(), row.value.clone());
                    }
                }
                RowKind::QueryParam => {
                    if row.enabled && !row.value.is_empty() {
                        query_params.insert(row.name.clone(), row.value.clone());
                    }
                }
                RowKind::Header => {
                    if row.enabled && !row.value.is_empty() {
                        headers.insert(row.name.clone(), row.value.clone());
                    }
                }
                RowKind::Body => {
                    body = serde_json::from_str(&row.value).ok();
                }
            }
        }

        RequestDef {
            method: self.method.clone(),
            base_url: self.base_url.clone(),
            path_template: self.path_template.clone(),
            path_params,
            query_params,
            headers,
            body,
        }
    }
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}
