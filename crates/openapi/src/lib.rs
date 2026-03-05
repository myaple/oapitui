use anyhow::{Context, Result};
use oapitui_config::TlsConfig;
use openapiv3::{
    OpenAPI, Operation, Parameter, ReferenceOr, RequestBody, Schema, SchemaKind, Type,
};
use serde_json::{json, Value};

pub use openapiv3;

/// A flattened, display-ready endpoint.
#[derive(Debug, Clone)]
pub struct Endpoint {
    pub method: String,
    pub path: String,
    pub summary: String,
    pub description: String,
    pub operation_id: Option<String>,
    pub tags: Vec<String>,
    pub parameters: Vec<ResolvedParam>,
    pub request_body: Option<ResolvedBody>,
    pub responses: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedParam {
    pub name: String,
    pub location: String, // "path" | "query" | "header"
    pub required: bool,
    pub description: String,
    pub example: Value,
    pub schema_type: String,
}

#[derive(Debug, Clone)]
pub struct ResolvedBody {
    pub content_type: String,
    pub example: Value,
    pub description: String,
    /// All available content types for this request body, with their examples.
    /// The first entry matches `content_type` / `example` above.
    pub alternatives: Vec<(String, Value)>,
}

/// Fetch and parse an OpenAPI spec from a URL or local file path.
///
/// If the source starts with `http://` or `https://`, it is fetched via HTTP.
/// Otherwise it is treated as a local file path.
pub async fn fetch_spec(url: &str, tls: &TlsConfig) -> Result<OpenAPI> {
    let text = if url.starts_with("http://") || url.starts_with("https://") {
        let client = build_client(tls)?;
        client
            .get(url)
            .send()
            .await
            .with_context(|| format!("fetching {url}"))?
            .text()
            .await?
    } else {
        // Treat as local file path — expand ~ to home dir
        let path = if url.starts_with("~/") {
            dirs::home_dir()
                .unwrap_or_default()
                .join(&url[2..])
        } else {
            std::path::PathBuf::from(url)
        };
        std::fs::read_to_string(&path)
            .with_context(|| format!("reading local spec file {}", path.display()))?
    };

    // Try JSON first, then YAML fallback
    let spec: OpenAPI = serde_json::from_str(&text).or_else(|_| {
        serde_yaml::from_str(&text).with_context(|| format!("parsing OpenAPI spec from {url}"))
    })?;
    Ok(spec)
}

fn build_client(tls: &TlsConfig) -> Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder().user_agent("oapitui/0.1");

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

/// Extract all endpoints from a parsed spec, resolving $refs inline.
pub fn extract_endpoints(spec: &OpenAPI) -> Vec<Endpoint> {
    let mut endpoints = Vec::new();

    for (path, path_item) in &spec.paths.paths {
        let path_item = match path_item {
            ReferenceOr::Item(i) => i,
            ReferenceOr::Reference { .. } => continue,
        };

        let ops: Vec<(&str, Option<&Operation>)> = vec![
            ("GET", path_item.get.as_ref()),
            ("POST", path_item.post.as_ref()),
            ("PUT", path_item.put.as_ref()),
            ("DELETE", path_item.delete.as_ref()),
            ("PATCH", path_item.patch.as_ref()),
            ("HEAD", path_item.head.as_ref()),
            ("OPTIONS", path_item.options.as_ref()),
            ("TRACE", path_item.trace.as_ref()),
        ];

        for (method, op_opt) in ops {
            if let Some(op) = op_opt {
                let parameters = collect_params(op, spec);
                let request_body = collect_body(op, spec);
                let responses = op
                    .responses
                    .responses
                    .keys()
                    .map(|k: &openapiv3::StatusCode| k.to_string())
                    .collect();

                endpoints.push(Endpoint {
                    method: method.to_string(),
                    path: path.clone(),
                    summary: op.summary.clone().unwrap_or_default(),
                    description: op.description.clone().unwrap_or_default(),
                    operation_id: op.operation_id.clone(),
                    tags: op.tags.clone(),
                    parameters,
                    request_body,
                    responses,
                });
            }
        }
    }

    endpoints
}

fn collect_params(op: &Operation, spec: &OpenAPI) -> Vec<ResolvedParam> {
    op.parameters
        .iter()
        .filter_map(|p| resolve_param(p, spec))
        .collect()
}

fn resolve_param(p: &ReferenceOr<Parameter>, spec: &OpenAPI) -> Option<ResolvedParam> {
    let param = match p {
        ReferenceOr::Item(i) => i,
        ReferenceOr::Reference { reference } => {
            // Simple $ref resolution from components/parameters
            let name = reference.split('/').next_back()?;
            spec.components
                .as_ref()?
                .parameters
                .get(name)
                .and_then(|r| {
                    if let ReferenceOr::Item(i) = r {
                        Some(i)
                    } else {
                        None
                    }
                })?
        }
    };

    let (name, location, required, description, schema) = match param {
        Parameter::Query { parameter_data, .. } => (
            parameter_data.name.clone(),
            "query".to_string(),
            parameter_data.required,
            parameter_data.description.clone().unwrap_or_default(),
            extract_param_schema(&parameter_data.format),
        ),
        Parameter::Path { parameter_data, .. } => (
            parameter_data.name.clone(),
            "path".to_string(),
            true,
            parameter_data.description.clone().unwrap_or_default(),
            extract_param_schema(&parameter_data.format),
        ),
        Parameter::Header { parameter_data, .. } => (
            parameter_data.name.clone(),
            "header".to_string(),
            parameter_data.required,
            parameter_data.description.clone().unwrap_or_default(),
            extract_param_schema(&parameter_data.format),
        ),
        Parameter::Cookie { parameter_data, .. } => (
            parameter_data.name.clone(),
            "cookie".to_string(),
            parameter_data.required,
            parameter_data.description.clone().unwrap_or_default(),
            extract_param_schema(&parameter_data.format),
        ),
    };

    let (example, schema_type) = if let Some(s) = schema {
        let t = schema_type_label(&s);
        let ex = generate_example(&s, spec, 0);
        (ex, t)
    } else {
        (json!(""), "string".to_string())
    };

    Some(ResolvedParam {
        name,
        location,
        required,
        description,
        example,
        schema_type,
    })
}

fn extract_param_schema(format: &openapiv3::ParameterSchemaOrContent) -> Option<Schema> {
    match format {
        openapiv3::ParameterSchemaOrContent::Schema(ror) => match ror {
            ReferenceOr::Item(s) => Some(s.clone()),
            ReferenceOr::Reference { .. } => None,
        },
        openapiv3::ParameterSchemaOrContent::Content(_) => None,
    }
}

fn collect_body(op: &Operation, spec: &OpenAPI) -> Option<ResolvedBody> {
    let body_ref = op.request_body.as_ref()?;
    let body: &RequestBody = match body_ref {
        ReferenceOr::Item(b) => b,
        ReferenceOr::Reference { reference } => {
            let name = reference.split('/').next_back()?;
            match spec.components.as_ref()?.request_bodies.get(name)? {
                ReferenceOr::Item(b) => b,
                _ => return None,
            }
        }
    };

    // Collect all content types with their examples
    let mut alternatives: Vec<(String, Value)> = Vec::new();
    for (ct, media) in &body.content {
        let example = media
            .schema
            .as_ref()
            .and_then(|s| match s {
                ReferenceOr::Item(schema) => Some(generate_example(schema, spec, 0)),
                ReferenceOr::Reference { reference } => {
                    resolve_schema_ref(reference, spec).map(|s| generate_example(&s, spec, 0))
                }
            })
            .unwrap_or(json!({}));
        alternatives.push((ct.clone(), example));
    }

    if alternatives.is_empty() {
        return None;
    }

    // Put application/json first if present, otherwise keep original order
    if let Some(idx) = alternatives.iter().position(|(ct, _)| ct == "application/json") {
        alternatives.swap(0, idx);
    }

    let (ct, example) = alternatives[0].clone();

    Some(ResolvedBody {
        content_type: ct,
        example,
        description: body.description.clone().unwrap_or_default(),
        alternatives,
    })
}

/// Walk a Schema and produce a filled-in example Value.
pub fn generate_example(schema: &Schema, spec: &OpenAPI, depth: usize) -> Value {
    if depth > 8 {
        return json!(null);
    }

    // Check for inline example
    if let Some(ex) = &schema.schema_data.example {
        return ex.clone();
    }

    match &schema.schema_kind {
        SchemaKind::Type(t) => match t {
            Type::String(s) => {
                if let Some(e) = s.enumeration.first().and_then(|v| v.as_ref()) {
                    return json!(e);
                }
                let fmt_str = format_example(&s.format);
                if fmt_str != "string" {
                    return json!(fmt_str);
                }
                json!("string")
            }
            Type::Integer(i) => {
                if let Some(e) = i.enumeration.first().and_then(|v| *v) {
                    return json!(e);
                }
                json!(0)
            }
            Type::Number(n) => {
                if let Some(e) = n.enumeration.first().and_then(|v| *v) {
                    return json!(e);
                }
                json!(0.0)
            }
            Type::Boolean(_) => json!(false),
            Type::Array(a) => {
                let item_example = a
                    .items
                    .as_ref()
                    .and_then(|i| match i {
                        ReferenceOr::Item(s) => Some(generate_example(s, spec, depth + 1)),
                        ReferenceOr::Reference { reference } => resolve_schema_ref(reference, spec)
                            .map(|s| generate_example(&s, spec, depth + 1)),
                    })
                    .unwrap_or(json!("item"));
                json!([item_example])
            }
            Type::Object(o) => {
                let mut map = serde_json::Map::new();
                for (prop_name, prop_ref) in &o.properties {
                    let val = match prop_ref {
                        ReferenceOr::Item(s) => generate_example(s, spec, depth + 1),
                        ReferenceOr::Reference { reference } => resolve_schema_ref(reference, spec)
                            .map(|s| generate_example(&s, spec, depth + 1))
                            .unwrap_or(json!(null)),
                    };
                    map.insert(prop_name.clone(), val);
                }
                Value::Object(map)
            }
        },
        SchemaKind::AllOf { all_of } => {
            let mut map = serde_json::Map::new();
            for ror in all_of {
                let val = match ror {
                    ReferenceOr::Item(s) => generate_example(s, spec, depth + 1),
                    ReferenceOr::Reference { reference } => resolve_schema_ref(reference, spec)
                        .map(|s| generate_example(&s, spec, depth + 1))
                        .unwrap_or(json!(null)),
                };
                if let Value::Object(m) = val {
                    map.extend(m);
                }
            }
            Value::Object(map)
        }
        SchemaKind::OneOf { one_of } | SchemaKind::AnyOf { any_of: one_of } => one_of
            .first()
            .map(|ror| match ror {
                ReferenceOr::Item(s) => generate_example(s, spec, depth + 1),
                ReferenceOr::Reference { reference } => resolve_schema_ref(reference, spec)
                    .map(|s| generate_example(&s, spec, depth + 1))
                    .unwrap_or(json!(null)),
            })
            .unwrap_or(json!(null)),
        SchemaKind::Not { .. } => json!(null),
        SchemaKind::Any(_) => json!(null),
    }
}

fn format_example(fmt: &openapiv3::VariantOrUnknownOrEmpty<openapiv3::StringFormat>) -> String {
    use openapiv3::{StringFormat, VariantOrUnknownOrEmpty};
    match fmt {
        VariantOrUnknownOrEmpty::Item(StringFormat::DateTime) => "2024-01-01T00:00:00Z".to_string(),
        VariantOrUnknownOrEmpty::Item(StringFormat::Date) => "2024-01-01".to_string(),
        VariantOrUnknownOrEmpty::Item(StringFormat::Password) => "secret".to_string(),
        VariantOrUnknownOrEmpty::Item(StringFormat::Byte) => "dGVzdA==".to_string(),
        VariantOrUnknownOrEmpty::Item(StringFormat::Binary) => "binary".to_string(),
        VariantOrUnknownOrEmpty::Unknown(s) if s == "uuid" => {
            "00000000-0000-0000-0000-000000000000".to_string()
        }
        VariantOrUnknownOrEmpty::Unknown(s) if s == "email" => "user@example.com".to_string(),
        VariantOrUnknownOrEmpty::Unknown(s) if s == "uri" || s == "url" => {
            "https://example.com".to_string()
        }
        VariantOrUnknownOrEmpty::Empty => "string".to_string(),
        _ => "string".to_string(),
    }
}

pub fn schema_type_label(schema: &Schema) -> String {
    match &schema.schema_kind {
        SchemaKind::Type(Type::String(_)) => "string".to_string(),
        SchemaKind::Type(Type::Integer(_)) => "integer".to_string(),
        SchemaKind::Type(Type::Number(_)) => "number".to_string(),
        SchemaKind::Type(Type::Boolean(_)) => "boolean".to_string(),
        SchemaKind::Type(Type::Array(_)) => "array".to_string(),
        SchemaKind::Type(Type::Object(_)) => "object".to_string(),
        SchemaKind::AllOf { .. } => "allOf".to_string(),
        SchemaKind::OneOf { .. } => "oneOf".to_string(),
        SchemaKind::AnyOf { .. } => "anyOf".to_string(),
        _ => "any".to_string(),
    }
}

pub fn resolve_schema_ref(reference: &str, spec: &OpenAPI) -> Option<Schema> {
    let name = reference.split('/').next_back()?;
    spec.components
        .as_ref()?
        .schemas
        .get(name)
        .and_then(|r| match r {
            ReferenceOr::Item(s) => Some(s.clone()),
            _ => None,
        })
}
