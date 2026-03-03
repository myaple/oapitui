use oapitui_client::ResponseResult;
use std::cell::Cell;

pub struct ResponseViewerState {
    pub status: u16,
    pub elapsed_ms: u128,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub scroll: u16,
    pub show_headers: bool,
    /// When `Some(s)`, the save-to-file dialog is open and `s` is the typed filename.
    pub save_dialog: Option<String>,
    /// Height of the visible body area — updated by the UI renderer each frame.
    pub page_size: Cell<u16>,
}

impl Default for ResponseViewerState {
    fn default() -> Self {
        Self {
            status: 0,
            elapsed_ms: 0,
            headers: Vec::new(),
            body: String::new(),
            scroll: 0,
            show_headers: false,
            save_dialog: None,
            page_size: Cell::new(10),
        }
    }
}

impl ResponseViewerState {
    pub fn from_response(r: ResponseResult) -> Self {
        let body = r
            .body_json
            .as_ref()
            .and_then(|j| serde_json::to_string_pretty(j).ok())
            .unwrap_or(r.body.clone());
        Self {
            status: r.status,
            elapsed_ms: r.elapsed.as_millis(),
            headers: r.headers,
            body,
            scroll: 0,
            show_headers: false,
            save_dialog: None,
            page_size: Cell::new(10),
        }
    }
}
