use oapitui_client::ResponseResult;

#[derive(Default)]
pub struct ResponseViewerState {
    pub status: u16,
    pub elapsed_ms: u128,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub scroll: u16,
    pub show_headers: bool,
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
        }
    }
}
