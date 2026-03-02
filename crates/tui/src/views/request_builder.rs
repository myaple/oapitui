use oaitui_client::RequestDef;
use oaitui_openapi::Endpoint;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub enum RowKind {
    PathParam,
    QueryParam,
    Header,
    Body,
}

#[derive(Debug, Clone)]
pub struct FieldRow {
    pub kind: RowKind,
    pub name: String,
    pub type_label: String,
    pub value: String,
    pub required: bool,
}

pub struct RequestBuilderState {
    pub method: String,
    pub base_url: String,
    pub path_template: String,
    pub rows: Vec<FieldRow>,
    pub selected: usize,
    pub editing: bool,
    pub cursor: usize, // char position within value string
}

impl Default for RequestBuilderState {
    fn default() -> Self {
        Self {
            method: "GET".to_string(),
            base_url: String::new(),
            path_template: String::new(),
            rows: vec![],
            selected: 0,
            editing: false,
            cursor: 0,
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
            });
        }

        for (k, v) in &default_headers {
            rows.push(FieldRow {
                kind: RowKind::Header,
                name: k.clone(),
                type_label: "string".to_string(),
                value: v.clone(),
                required: false,
            });
        }

        if let Some(body) = &ep.request_body {
            let pretty = serde_json::to_string_pretty(&body.example).unwrap_or_default();
            rows.push(FieldRow {
                kind: RowKind::Body,
                name: "body".to_string(),
                type_label: body.content_type.clone(),
                value: pretty,
                required: false,
            });
        }

        let cursor = rows.first().map(|r| r.value.len()).unwrap_or(0);

        Self {
            method: ep.method.clone(),
            base_url,
            path_template: ep.path.clone(),
            rows,
            selected: 0,
            editing: false,
            cursor,
        }
    }

    pub fn next_row(&mut self) {
        if !self.rows.is_empty() {
            self.selected = (self.selected + 1).min(self.rows.len() - 1);
            self.cursor = self.rows[self.selected].value.len();
        }
    }

    pub fn prev_row(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.cursor = self.rows[self.selected].value.len();
        }
    }

    pub fn edit_char(&mut self, c: char) {
        if let Some(row) = self.rows.get_mut(self.selected) {
            // Insert at cursor position (byte-safe for ASCII; good enough for URLs/JSON)
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

    pub fn build_request(&self) -> RequestDef {
        let mut path_params = HashMap::new();
        let mut query_params = HashMap::new();
        let mut headers = HashMap::new();
        let mut body: Option<Value> = None;

        for row in &self.rows {
            match row.kind {
                RowKind::PathParam => {
                    path_params.insert(row.name.clone(), row.value.clone());
                }
                RowKind::QueryParam => {
                    if !row.value.is_empty() {
                        query_params.insert(row.name.clone(), row.value.clone());
                    }
                }
                RowKind::Header => {
                    if !row.value.is_empty() {
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
