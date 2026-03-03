use crate::theme::Theme;
use crate::views::{
    add_server::AddServerState, endpoint_list::EndpointListState,
    request_builder::RequestBuilderState, response_viewer::ResponseViewerState,
    server_list::ServerListState,
};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use oapitui_client::ResponseResult;
use oapitui_config::{Config, ServerEntry};
use oapitui_openapi::{extract_endpoints, fetch_spec, openapiv3::OpenAPI};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    collections::HashMap,
    io,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};

/// Messages sent from background tasks back to the main loop.
pub enum BgMsg {
    SpecLoaded {
        server_name: String,
        spec: Box<OpenAPI>,
    },
    SpecError {
        server_name: String,
        error: String,
    },
    ResponseReady(ResponseResult),
    ResponseError(String),
}

pub enum Screen {
    ServerList,
    AddServer,
    EndpointList,
    RequestBuilder,
    ResponseViewer,
}

pub struct App {
    pub config: Config,
    pub config_path: Option<PathBuf>,
    pub theme: Theme,

    // Per-server cached specs
    pub specs: HashMap<String, Arc<OpenAPI>>,
    pub spec_loading: HashMap<String, bool>,
    pub last_refreshed: HashMap<String, Instant>,

    // Background message channel
    pub tx: UnboundedSender<BgMsg>,
    pub rx: UnboundedReceiver<BgMsg>,

    // Current screen
    pub screen: Screen,

    // Per-screen state
    pub server_list: ServerListState,
    pub add_server: AddServerState,
    pub endpoint_list: EndpointListState,
    pub request_builder: RequestBuilderState,
    pub response_viewer: ResponseViewerState,

    // Transient error banner
    pub error: Option<String>,

    pub should_quit: bool,
}

impl App {
    pub fn new(config_path: Option<PathBuf>) -> Result<Self> {
        let path_ref = config_path.as_ref().or(None);
        let config = Config::load(path_ref)?;
        let theme = Theme::from_config(&config.theme);
        let (tx, rx) = mpsc::unbounded_channel();

        Ok(Self {
            config,
            config_path,
            theme,
            specs: HashMap::new(),
            spec_loading: HashMap::new(),
            last_refreshed: HashMap::new(),
            tx,
            rx,
            screen: Screen::ServerList,
            server_list: ServerListState::default(),
            add_server: AddServerState::default(),
            endpoint_list: EndpointListState::default(),
            request_builder: RequestBuilderState::default(),
            response_viewer: ResponseViewerState::default(),
            error: None,
            should_quit: false,
        })
    }

    pub async fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    ) -> Result<()> {
        // Pre-fetch all specs in background
        let to_fetch: Vec<(String, String, oapitui_config::TlsConfig)> = self
            .config
            .servers
            .iter()
            .map(|s| (s.name.clone(), s.url.clone(), s.tls.clone()))
            .collect();
        for (name, url, tls) in to_fetch {
            self.spawn_fetch(name, url, tls);
        }

        loop {
            terminal.draw(|f| crate::ui::render(f, self))?;

            // Drain background messages (non-blocking)
            while let Ok(msg) = self.rx.try_recv() {
                self.handle_bg(msg);
            }

            // Poll for events with a 100ms timeout so we keep draining bg messages
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    self.handle_key(key.code, key.modifiers).await?;
                }
            }

            if self.should_quit {
                break;
            }
        }
        Ok(())
    }

    fn handle_bg(&mut self, msg: BgMsg) {
        match msg {
            BgMsg::SpecLoaded { server_name, spec } => {
                self.spec_loading.remove(&server_name);
                self.last_refreshed
                    .insert(server_name.clone(), Instant::now());
                self.specs.insert(server_name, Arc::new(*spec));
            }
            BgMsg::SpecError { server_name, error } => {
                self.spec_loading.remove(&server_name);
                self.error = Some(format!("Failed to load spec for '{server_name}': {error}"));
            }
            BgMsg::ResponseReady(resp) => {
                self.response_viewer = ResponseViewerState::from_response(resp);
                self.screen = Screen::ResponseViewer;
            }
            BgMsg::ResponseError(e) => {
                self.error = Some(format!("Request failed: {e}"));
            }
        }
    }

    pub async fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) -> Result<()> {
        // Ctrl-C / Ctrl-Q always quit
        if matches!(
            (code, modifiers),
            (KeyCode::Char('c'), KeyModifiers::CONTROL)
                | (KeyCode::Char('q'), KeyModifiers::CONTROL)
        ) {
            self.should_quit = true;
            return Ok(());
        }

        // Dismiss error banner with any key
        if self.error.is_some() && code != KeyCode::Null {
            self.error = None;
            return Ok(());
        }

        match self.screen {
            Screen::ServerList => self.handle_server_list(code, modifiers).await?,
            Screen::AddServer => self.handle_add_server(code),
            Screen::EndpointList => self.handle_endpoint_list(code),
            Screen::RequestBuilder => self.handle_request_builder(code, modifiers).await?,
            Screen::ResponseViewer => self.handle_response_viewer(code),
        }
        Ok(())
    }

    async fn handle_server_list(&mut self, code: KeyCode, _m: KeyModifiers) -> Result<()> {
        let n = self.config.servers.len();
        match code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => {
                if n > 0 {
                    self.server_list.selected = (self.server_list.selected + 1).min(n - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.server_list.selected > 0 {
                    self.server_list.selected -= 1;
                }
            }
            KeyCode::Home => {
                self.server_list.selected = 0;
            }
            KeyCode::End => {
                if n > 0 {
                    self.server_list.selected = n - 1;
                }
            }
            KeyCode::PageUp => {
                let page = self.server_list.page_size.get().max(1) as usize;
                self.server_list.selected = self.server_list.selected.saturating_sub(page);
            }
            KeyCode::PageDown => {
                if n > 0 {
                    let page = self.server_list.page_size.get().max(1) as usize;
                    self.server_list.selected = (self.server_list.selected + page).min(n - 1);
                }
            }
            KeyCode::Char('a') => {
                self.add_server = AddServerState::default();
                self.screen = Screen::AddServer;
            }
            KeyCode::Char('d') => {
                if n > 0 {
                    let name = self.config.servers[self.server_list.selected].name.clone();
                    self.config.remove_server(&name);
                    self.specs.remove(&name);
                    let _ = self.config.save(self.config_path.as_ref());
                    if self.server_list.selected >= self.config.servers.len()
                        && self.server_list.selected > 0
                    {
                        self.server_list.selected -= 1;
                    }
                }
            }
            KeyCode::Char('r') => {
                if n > 0 {
                    let s = &self.config.servers[self.server_list.selected];
                    self.spawn_fetch(s.name.clone(), s.url.clone(), s.tls.clone());
                }
            }
            KeyCode::Enter => {
                if n > 0 {
                    let name = &self.config.servers[self.server_list.selected].name;
                    if let Some(spec) = self.specs.get(name) {
                        let endpoints = extract_endpoints(spec);
                        let spec_url = &self.config.servers[self.server_list.selected].url;
                        let server_base = spec
                            .servers
                            .first()
                            .map(|s| resolve_server_base(spec_url, &s.url))
                            .unwrap_or_default();
                        self.endpoint_list =
                            EndpointListState::new(endpoints, name.clone(), server_base);
                        self.screen = Screen::EndpointList;
                    } else {
                        self.error = Some(format!(
                            "Spec for '{}' not loaded yet — press 'r' to retry",
                            name
                        ));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_add_server(&mut self, code: KeyCode) {
        use crate::views::add_server::AddServerField;
        match code {
            KeyCode::Esc => self.screen = Screen::ServerList,
            KeyCode::Tab => {
                self.add_server.field = match self.add_server.field {
                    AddServerField::Name => AddServerField::Url,
                    AddServerField::Url => AddServerField::ClientCert,
                    AddServerField::ClientCert => AddServerField::ClientKey,
                    AddServerField::ClientKey => AddServerField::CaCert,
                    AddServerField::CaCert => AddServerField::Name,
                };
            }
            KeyCode::Enter => {
                let name = self.add_server.name.trim().to_string();
                let url = self.add_server.url.trim().to_string();
                if !name.is_empty() && !url.is_empty() {
                    let tls = oapitui_config::TlsConfig {
                        client_cert: non_empty(&self.add_server.client_cert),
                        client_key: non_empty(&self.add_server.client_key),
                        ca_cert: non_empty(&self.add_server.ca_cert),
                    };
                    let entry = ServerEntry {
                        name: name.clone(),
                        url: url.clone(),
                        description: String::new(),
                        default_headers: Default::default(),
                        tls: tls.clone(),
                    };
                    self.config.add_server(entry);
                    let _ = self.config.save(self.config_path.as_ref());
                    self.spawn_fetch(name, url, tls);
                    self.screen = Screen::ServerList;
                }
            }
            KeyCode::Backspace => {
                use AddServerField::*;
                match self.add_server.field {
                    Name => {
                        self.add_server.name.pop();
                    }
                    Url => {
                        self.add_server.url.pop();
                    }
                    ClientCert => {
                        self.add_server.client_cert.pop();
                    }
                    ClientKey => {
                        self.add_server.client_key.pop();
                    }
                    CaCert => {
                        self.add_server.ca_cert.pop();
                    }
                }
            }
            KeyCode::Char(c) => {
                use AddServerField::*;
                match self.add_server.field {
                    Name => self.add_server.name.push(c),
                    Url => self.add_server.url.push(c),
                    ClientCert => self.add_server.client_cert.push(c),
                    ClientKey => self.add_server.client_key.push(c),
                    CaCert => self.add_server.ca_cert.push(c),
                }
            }
            _ => {}
        }
    }

    fn handle_endpoint_list(&mut self, code: KeyCode) {
        let el = &mut self.endpoint_list;

        // Curl popup: Esc or 'c' closes it; all other keys are swallowed
        if el.show_curl {
            match code {
                KeyCode::Esc | KeyCode::Char('c') => el.show_curl = false,
                _ => {}
            }
            return;
        }

        // Detail panel focus: j/k scroll the detail, Tab or Esc unfocuses
        if el.detail_focused {
            match code {
                KeyCode::Char('j') | KeyCode::Down => {
                    el.detail_scroll = el.detail_scroll.saturating_add(1);
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    el.detail_scroll = el.detail_scroll.saturating_sub(1);
                }
                KeyCode::Tab | KeyCode::Esc => {
                    el.detail_focused = false;
                }
                _ => {}
            }
            return;
        }

        match code {
            KeyCode::Esc if el.filter_active => {
                el.filter_active = false;
                el.filter.clear();
                el.selected = 0;
            }
            KeyCode::Esc if !el.filter.is_empty() => {
                el.filter.clear();
                el.selected = 0;
            }
            KeyCode::Esc => self.screen = Screen::ServerList,
            KeyCode::Char('q') if !el.filter_active => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down if !el.filter_active => el.next(),
            KeyCode::Char('k') | KeyCode::Up if !el.filter_active => el.prev(),
            KeyCode::Home if !el.filter_active => el.home(),
            KeyCode::End if !el.filter_active => el.end(),
            KeyCode::PageUp if !el.filter_active => el.page_up(),
            KeyCode::PageDown if !el.filter_active => el.page_down(),
            KeyCode::Tab if !el.filter_active => {
                el.detail_focused = true;
                el.detail_scroll = 0;
            }
            KeyCode::Char('s') if !el.filter_active => {
                el.sort_mode = el.sort_mode.next();
                el.selected = 0;
            }
            KeyCode::Char('c') if !el.filter_active => {
                if el.selected_endpoint().is_some() {
                    el.show_curl = true;
                }
            }
            KeyCode::Char('/') if !el.filter_active => {
                el.filter_active = true;
                el.filter.clear();
                el.selected = 0;
            }
            KeyCode::Backspace if el.filter_active => {
                el.filter.pop();
                el.selected = 0;
            }
            KeyCode::Char(c) if el.filter_active => {
                el.filter.push(c);
                el.selected = 0;
            }
            KeyCode::Enter if el.filter_active => {
                el.filter_active = false;
            }
            KeyCode::Enter => {
                if let Some(ep) = el.selected_endpoint() {
                    let server_name = el.server_name.clone();
                    let base_url = el.server_base.clone();
                    // Find default headers from config
                    let default_headers = self
                        .config
                        .servers
                        .iter()
                        .find(|s| s.name == server_name)
                        .map(|s| s.default_headers.clone())
                        .unwrap_or_default();
                    self.request_builder =
                        RequestBuilderState::from_endpoint(ep, base_url, default_headers);
                    self.screen = Screen::RequestBuilder;
                }
            }
            _ => {}
        }
    }

    async fn handle_request_builder(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
    ) -> Result<()> {
        use crate::views::request_builder::FocusedPane;
        let rb = &mut self.request_builder;

        match rb.focus {
            // ── Top pane: navigating param rows ──────────────────────────────
            FocusedPane::ParamsNav => match code {
                KeyCode::Esc => self.screen = Screen::EndpointList,
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Char('j') | KeyCode::Down => rb.next_row(),
                KeyCode::Char('k') | KeyCode::Up => rb.prev_row(),
                KeyCode::Char(' ') => rb.toggle_enabled(),
                KeyCode::Char('e') => {
                    // Focus the param value for editing; place cursor at end.
                    rb.cursor = rb
                        .rows
                        .get(rb.selected)
                        .map(|r| r.value.chars().count())
                        .unwrap_or(0);
                    rb.focus = FocusedPane::ParamsEdit;
                }
                KeyCode::Tab if rb.has_body() => {
                    rb.focus = FocusedPane::BodyNormal;
                }
                KeyCode::Enter => {
                    let req = rb.build_request();
                    let tls = self
                        .config
                        .servers
                        .iter()
                        .find(|s| s.name == self.endpoint_list.server_name)
                        .map(|s| s.tls.clone())
                        .unwrap_or_default();
                    let tx = self.tx.clone();
                    tokio::spawn(async move {
                        match oapitui_client::execute(&req, &tls).await {
                            Ok(resp) => {
                                let _ = tx.send(BgMsg::ResponseReady(resp));
                            }
                            Err(e) => {
                                let _ = tx.send(BgMsg::ResponseError(format!("{:#}", e)));
                            }
                        }
                    });
                }
                _ => {}
            },

            // ── Top pane: editing a param value inline ────────────────────────
            FocusedPane::ParamsEdit => match code {
                KeyCode::Esc => rb.focus = FocusedPane::ParamsNav,
                KeyCode::Backspace => rb.edit_backspace(),
                KeyCode::Char(c) => rb.edit_char(c),
                _ => {}
            },

            // ── Bottom pane: vim normal mode ──────────────────────────────────
            FocusedPane::BodyNormal => {
                let pending = rb.pending_key.take();
                match (pending, code) {
                    // Two-key sequences
                    (Some('g'), KeyCode::Char('g')) => rb.body_goto_top(),
                    (Some('d'), KeyCode::Char('d')) => rb.body_delete_line(),
                    (Some('d'), KeyCode::Char('w')) => rb.body_delete_word(),
                    // Unknown second key — cancel silently.
                    (Some(_), _) => {}

                    // Single-key commands
                    (None, KeyCode::Esc) | (None, KeyCode::Tab) => {
                        rb.focus = FocusedPane::ParamsNav;
                    }
                    (None, KeyCode::Char('h')) | (None, KeyCode::Left) => rb.cursor_left(),
                    (None, KeyCode::Char('l')) | (None, KeyCode::Right) => rb.cursor_right(),
                    (None, KeyCode::Char('k')) | (None, KeyCode::Up) => rb.cursor_up(),
                    (None, KeyCode::Char('j')) | (None, KeyCode::Down) => rb.cursor_down(),
                    (None, KeyCode::Char('0')) => rb.cursor_line_start(),
                    (None, KeyCode::Char('$')) => rb.cursor_line_end(),
                    (None, KeyCode::Char('G')) => rb.body_goto_bottom(),
                    // First key of a two-key sequence
                    (None, KeyCode::Char('g')) => rb.pending_key = Some('g'),
                    (None, KeyCode::Char('d')) => rb.pending_key = Some('d'),
                    // Enter insert mode
                    (None, KeyCode::Char('i')) => rb.focus = FocusedPane::BodyInsert,
                    (None, KeyCode::Char('a')) => {
                        rb.cursor_right();
                        rb.focus = FocusedPane::BodyInsert;
                    }
                    (None, KeyCode::Char('I')) => {
                        rb.cursor_line_start();
                        rb.focus = FocusedPane::BodyInsert;
                    }
                    (None, KeyCode::Char('A')) => {
                        rb.cursor_line_end();
                        rb.focus = FocusedPane::BodyInsert;
                    }
                    _ => {}
                }
            }

            // ── Bottom pane: insert mode ──────────────────────────────────────
            FocusedPane::BodyInsert => match code {
                KeyCode::Esc => rb.focus = FocusedPane::BodyNormal,
                KeyCode::Backspace => rb.edit_body_backspace(),
                KeyCode::Enter => rb.edit_body_char('\n'),
                KeyCode::Char(c) => rb.edit_body_char(c),
                _ => {}
            },
        }
        Ok(())
    }

    fn handle_response_viewer(&mut self, code: KeyCode) {
        let rv = &mut self.response_viewer;

        // Save dialog: handle filename input
        if rv.save_dialog.is_some() {
            match code {
                KeyCode::Esc => rv.save_dialog = None,
                KeyCode::Backspace => {
                    if let Some(ref mut name) = rv.save_dialog {
                        name.pop();
                    }
                }
                KeyCode::Char(c) => {
                    if let Some(ref mut name) = rv.save_dialog {
                        name.push(c);
                    }
                }
                KeyCode::Enter => {
                    if let Some(ref name) = rv.save_dialog {
                        let path = name.trim().to_string();
                        if !path.is_empty() {
                            if let Err(e) = std::fs::write(&path, &rv.body) {
                                self.error = Some(format!("Failed to save '{path}': {e}"));
                            }
                        }
                    }
                    rv.save_dialog = None;
                }
                _ => {}
            }
            return;
        }

        match code {
            KeyCode::Esc => self.screen = Screen::RequestBuilder,
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('j') | KeyCode::Down => rv.scroll = rv.scroll.saturating_add(1),
            KeyCode::Char('k') | KeyCode::Up => rv.scroll = rv.scroll.saturating_sub(1),
            KeyCode::PageDown => {
                let page = rv.page_size.get().max(1);
                rv.scroll = rv.scroll.saturating_add(page);
            }
            KeyCode::PageUp => {
                let page = rv.page_size.get().max(1);
                rv.scroll = rv.scroll.saturating_sub(page);
            }
            KeyCode::Home => rv.scroll = 0,
            KeyCode::End => {
                let total = rv.body.lines().count() as u16;
                rv.scroll = total.saturating_sub(rv.page_size.get());
            }
            KeyCode::Char('h') => rv.show_headers = !rv.show_headers,
            KeyCode::Char('s') => rv.save_dialog = Some(String::new()),
            _ => {}
        }
    }

    fn spawn_fetch(&mut self, name: String, url: String, tls: oapitui_config::TlsConfig) {
        self.spec_loading.insert(name.clone(), true);
        let tx = self.tx.clone();
        tokio::spawn(async move {
            match fetch_spec(&url, &tls).await {
                Ok(spec) => {
                    let _ = tx.send(BgMsg::SpecLoaded {
                        server_name: name,
                        spec: Box::new(spec),
                    });
                }
                Err(e) => {
                    let _ = tx.send(BgMsg::SpecError {
                        server_name: name,
                        error: e.to_string(),
                    });
                }
            }
        });
    }
}

/// Resolve a server URL from an OpenAPI spec against the URL used to fetch the spec.
/// If `server_url` is already absolute (starts with a scheme), it is returned as-is.
/// If it is relative (e.g. `/api/v1`), the scheme and host are taken from `spec_url`.
fn resolve_server_base(spec_url: &str, server_url: &str) -> String {
    if server_url.starts_with("http://") || server_url.starts_with("https://") {
        return server_url.to_string();
    }
    // Extract scheme://host from the spec URL and prepend it to the relative server URL.
    if let Some(scheme_end) = spec_url.find("://") {
        let after_scheme = &spec_url[scheme_end + 3..];
        let host_end = after_scheme.find('/').unwrap_or(after_scheme.len());
        let origin = &spec_url[..scheme_end + 3 + host_end];
        let path = if server_url.starts_with('/') {
            server_url.to_string()
        } else {
            format!("/{server_url}")
        };
        return format!("{origin}{path}");
    }
    server_url.to_string()
}

fn non_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

/// Build a template curl command for an endpoint.
pub fn build_curl_command(server_base: &str, ep: &oapitui_openapi::Endpoint) -> String {
    let base = server_base.trim_end_matches('/');
    let url = format!("{base}{}", ep.path);

    let mut lines: Vec<String> = vec![format!("curl -X {} \\", ep.method)];
    lines.push(format!("  '{url}' \\"));

    for param in &ep.parameters {
        if param.location == "header" {
            lines.push(format!("  -H '{}: <value>' \\", param.name));
        }
    }

    if let Some(body) = &ep.request_body {
        lines.push(format!("  -H 'Content-Type: {}' \\", body.content_type));
        let example =
            serde_json::to_string_pretty(&body.example).unwrap_or_else(|_| "{}".to_string());
        // Escape single quotes in the body example
        let escaped = example.replace('\'', "'\\''");
        lines.push(format!("  -d '{escaped}'"));
    } else {
        // Remove trailing backslash from the last line
        if let Some(last) = lines.last_mut() {
            *last = last.trim_end_matches(" \\").to_string();
        }
    }

    lines.join("\n")
}
