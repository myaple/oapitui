use anyhow::{Context, Result};
use oaitui_config::TlsConfig;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default)]
pub struct RequestDef {
    pub method: String,
    pub base_url: String,
    pub path_template: String,
    pub path_params: HashMap<String, String>,
    pub query_params: HashMap<String, String>,
    pub headers: HashMap<String, String>,
    pub body: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct ResponseResult {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: String,
    pub body_json: Option<Value>,
    pub elapsed: Duration,
}

impl RequestDef {
    /// Substitute `{param}` placeholders in the path template.
    pub fn resolved_url(&self) -> String {
        let mut path = self.path_template.clone();
        for (k, v) in &self.path_params {
            path = path.replace(&format!("{{{k}}}"), v);
        }
        let base = self.base_url.trim_end_matches('/');
        let path = path.trim_start_matches('/');
        format!("{base}/{path}")
    }
}

pub async fn execute(req: &RequestDef, tls: &TlsConfig) -> Result<ResponseResult> {
    let client = build_client(tls)?;

    let url = req.resolved_url();
    let method = reqwest::Method::from_bytes(req.method.to_uppercase().as_bytes())
        .with_context(|| format!("invalid method {}", req.method))?;

    let mut builder = client.request(method, &url);

    // Query params
    if !req.query_params.is_empty() {
        builder = builder.query(
            &req.query_params
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect::<Vec<_>>(),
        );
    }

    // Headers
    for (k, v) in &req.headers {
        builder = builder.header(k.as_str(), v.as_str());
    }

    // Body
    if let Some(body) = &req.body {
        builder = builder
            .header("content-type", "application/json")
            .json(body);
    }

    let start = Instant::now();
    let response = builder.send().await.context("sending request")?;
    let elapsed = start.elapsed();

    let status = response.status().as_u16();
    let headers: Vec<(String, String)> = response
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("<binary>").to_string()))
        .collect();

    let body = response.text().await.unwrap_or_default();
    let body_json = serde_json::from_str(&body).ok();

    Ok(ResponseResult {
        status,
        headers,
        body,
        body_json,
        elapsed,
    })
}

fn build_client(tls: &TlsConfig) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .user_agent("oaitui/0.1")
        .timeout(Duration::from_secs(30));

    if let (Some(cert_path), Some(key_path)) = (&tls.client_cert, &tls.client_key) {
        let mut pem =
            std::fs::read(cert_path).with_context(|| format!("reading client cert {cert_path}"))?;
        pem.extend(
            std::fs::read(key_path).with_context(|| format!("reading client key {key_path}"))?,
        );
        builder = builder.identity(reqwest::Identity::from_pem(&pem)?);
    }

    if let Some(ca_path) = &tls.ca_cert {
        let pem = std::fs::read(ca_path).with_context(|| format!("reading CA cert {ca_path}"))?;
        builder = builder.add_root_certificate(reqwest::Certificate::from_pem(&pem)?);
    }

    Ok(builder.build()?)
}
