use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// All theme color overrides. Each field accepts a color name (e.g. `"cyan"`,
/// `"dark_gray"`) or a hex value (e.g. `"#1e1e2e"`). Omitted fields use the
/// built-in defaults.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThemeConfig {
    // HTTP method badge colors
    pub method_get: Option<String>,
    pub method_post: Option<String>,
    pub method_put: Option<String>,
    pub method_delete: Option<String>,
    pub method_patch: Option<String>,
    pub method_other: Option<String>,

    // HTTP status-code range colors
    pub status_2xx: Option<String>,
    pub status_3xx: Option<String>,
    pub status_4xx: Option<String>,
    pub status_5xx: Option<String>,
    pub status_other: Option<String>,

    // UI chrome
    pub title: Option<String>,
    pub selected_bg: Option<String>,
    pub border_focused: Option<String>,
    pub border_unfocused: Option<String>,
    pub border_active: Option<String>,
    pub border_editing: Option<String>,

    // Text roles
    pub text_primary: Option<String>,
    pub text_secondary: Option<String>,
    pub text_url: Option<String>,
    pub text_key: Option<String>,
    pub text_tag: Option<String>,
    pub text_accent: Option<String>,

    // Status indicators (loading / success / error icons)
    pub indicator_loading: Option<String>,
    pub indicator_success: Option<String>,
    pub indicator_error: Option<String>,

    // Help bar
    pub help_key: Option<String>,
    pub help_desc: Option<String>,

    // Error banner
    pub error: Option<String>,

    // JSON syntax highlighting
    pub json_string: Option<String>,
    pub json_number: Option<String>,
    pub json_bool: Option<String>,
    pub json_null: Option<String>,

    // Markdown rendering
    pub md_h1: Option<String>,
    pub md_h2: Option<String>,
    pub md_code: Option<String>,
    pub md_quote: Option<String>,

    // Parameter list
    pub param_required: Option<String>,
    pub param_location: Option<String>,
    pub param_type: Option<String>,
    pub param_example: Option<String>,

    // Body editor cursors
    pub cursor_block_fg: Option<String>,
    pub cursor_block_bg: Option<String>,
    pub cursor_bar: Option<String>,

    // Endpoint filter bar
    pub filter_active: Option<String>,
    pub filter_inactive: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub servers: Vec<ServerEntry>,
    #[serde(default)]
    pub theme: ThemeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerEntry {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub default_headers: HashMap<String, String>,
    #[serde(default)]
    pub tls: TlsConfig,
}

/// Mutual-TLS and custom-CA configuration for a server.
///
/// ```toml
/// [[servers]]
/// name = "My API"
/// url  = "https://api.example.com/openapi.json"
///
/// [servers.tls]
/// client_cert = "/path/to/client.crt"   # PEM — client certificate
/// client_key  = "/path/to/client.key"   # PEM — client private key
/// ca_cert     = "/path/to/ca.crt"       # PEM — custom CA for server verification
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TlsConfig {
    /// Path to the client certificate PEM (required for mTLS).
    pub client_cert: Option<String>,
    /// Path to the client private key PEM (required for mTLS).
    pub client_key: Option<String>,
    /// Path to a custom CA certificate PEM used to verify the server.
    pub ca_cert: Option<String>,
}

impl Config {
    pub fn load(path: Option<&PathBuf>) -> Result<Self> {
        let p = match path {
            Some(p) => p.clone(),
            None => default_config_path(),
        };
        if !p.exists() {
            return Ok(Config::default());
        }
        let text = std::fs::read_to_string(&p)
            .with_context(|| format!("reading config {}", p.display()))?;
        let cfg: Config =
            toml::from_str(&text).with_context(|| format!("parsing config {}", p.display()))?;
        Ok(cfg)
    }

    pub fn save(&self, path: Option<&PathBuf>) -> Result<()> {
        let p = match path {
            Some(p) => p.clone(),
            None => default_config_path(),
        };
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let text = toml::to_string_pretty(self)?;
        std::fs::write(&p, text).with_context(|| format!("writing config {}", p.display()))?;
        Ok(())
    }

    pub fn add_server(&mut self, entry: ServerEntry) {
        self.servers.retain(|s| s.name != entry.name);
        self.servers.push(entry);
    }

    pub fn remove_server(&mut self, name: &str) {
        self.servers.retain(|s| s.name != name);
    }
}

pub fn default_config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("oapitui")
        .join("config.toml")
}
