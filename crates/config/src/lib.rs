use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub servers: Vec<ServerEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerEntry {
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub default_headers: HashMap<String, String>,
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
        .join("oaitui")
        .join("config.toml")
}
