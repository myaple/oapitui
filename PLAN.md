# oaitui — OpenAPI TUI Implementation Plan

## Overview

A terminal UI application written in Rust that reads a config file of OpenAPI server
definitions, lets users browse/add servers, explore endpoints and schemas, and send
live HTTP requests with schema-driven value editors.

---

## Workspace Layout

```
oaitui/
├── Cargo.toml                  # workspace manifest
├── config.example.toml         # example user config
└── crates/
    ├── config/                 # crate: oaitui-config
    │   ├── Cargo.toml
    │   └── src/lib.rs          # Config struct, load/save, add server
    ├── openapi/                # crate: oaitui-openapi
    │   ├── Cargo.toml
    │   └── src/lib.rs          # OpenAPI parsing, example-value generation
    ├── client/                 # crate: oaitui-client
    │   ├── Cargo.toml
    │   └── src/lib.rs          # Async HTTP request execution
    └── tui/                    # crate: oaitui-tui  (the binary)
        ├── Cargo.toml
        └── src/
            ├── main.rs         # tokio entry point
            ├── app.rs          # App state machine + event loop
            ├── ui.rs           # top-level render dispatcher
            └── views/
                ├── server_list.rs
                ├── endpoint_list.rs
                ├── request_builder.rs
                └── response_viewer.rs
```

---

## Crate Responsibilities

### `oaitui-config`
- **Config file format**: TOML at `~/.config/oaitui/config.toml` (or path from `--config`)
- **Server entry fields**: `name`, `url` (URL to openapi.json), optional `description`,
  optional `default_headers` (map of header key→value)
- **Operations**: `load()`, `save()`, `add_server()`, `remove_server()`
- **Dependencies**: `serde`, `serde_json`, `toml`, `dirs`

### `oaitui-openapi`
- **Parse**: fetch and parse OpenAPI 3.x JSON specs
- **Model types**: `ApiSpec`, `Server`, `PathItem`, `Operation`, `Parameter`,
  `RequestBody`, `Schema`, `SchemaKind` (object/array/primitive)
- **Example generation**: `fn generate_example(schema: &Schema) -> serde_json::Value`
  — walks the schema tree to produce a filled-in example value
- **Dependencies**: `openapiv3`, `serde_json`, `reqwest` (for fetching)

### `oaitui-client`
- **`RequestDef`**: method, url, headers, path params, query params, body JSON
- **`fn execute(req: RequestDef) -> ResponseResult`**: sends the request, returns
  status, headers, body, elapsed time
- **Dependencies**: `reqwest`, `tokio`, `serde_json`

### `oaitui-tui` (binary)
- Owns the terminal, event loop, and all rendering
- App state is a flat enum of screens + the data each screen needs
- Uses `ratatui` + `crossterm` for rendering and keyboard/mouse input
- Spawns a `tokio` runtime; async operations (fetch spec, send request) run as
  background tasks whose results are sent back via a channel

---

## TUI Screen Flow

```
ServerList ──(Enter)──► EndpointList ──(Enter)──► RequestBuilder ──(Enter)──► ResponseViewer
    │                        │                          │                          │
    │                    (Esc back)                 (Esc back)                (Esc back)
    │
  (a) AddServer modal (inline popup)
```

### Screen Descriptions

| Screen | Key bindings | What it shows |
|---|---|---|
| **ServerList** | `j/k` navigate, `Enter` select, `a` add, `d` delete, `r` refresh spec, `q` quit | List of configured servers with name, URL, spec load status |
| **AddServer modal** | Type name/URL, `Tab` between fields, `Enter` confirm, `Esc` cancel | Two text inputs: name + openapi.json URL |
| **EndpointList** | `j/k` navigate, `/` filter, `Enter` open, `Esc` back | All paths grouped by tag; shows method badge + summary |
| **RequestBuilder** | `Tab` cycle fields, `e` edit value, `Enter` send, `Esc` back | Editable table of path params, query params, headers, body JSON |
| **ResponseViewer** | `j/k` scroll, `Esc` back | Status line, response headers (collapsible), pretty-printed JSON body |

---

## MVP Feature Checklist (Build Order)

### Phase 1 — Scaffold & Config
- [ ] Create workspace `Cargo.toml`
- [ ] Create `oaitui-config` crate with `load`/`save`/`add_server`
- [ ] Create `config.example.toml` with a public OpenAPI server (e.g. petstore)
- [ ] Write unit tests for config load/save round-trip

### Phase 2 — OpenAPI Parsing
- [ ] Create `oaitui-openapi` crate
- [ ] Fetch spec JSON from URL via `reqwest`
- [ ] Parse with `openapiv3` crate into internal types
- [ ] Implement `generate_example(schema)` for objects, arrays, primitives, `$ref`
- [ ] Write unit tests against a bundled petstore spec fixture

### Phase 3 — HTTP Client
- [ ] Create `oaitui-client` crate
- [ ] `RequestDef` struct + `execute()` function
- [ ] Substitute path parameters into URL template
- [ ] Append query params
- [ ] Serialize body to JSON
- [ ] Return structured `ResponseResult`

### Phase 4 — TUI Shell
- [ ] Create `oaitui-tui` binary crate
- [ ] Set up `crossterm` raw mode + `ratatui` terminal
- [ ] Main event loop: poll keyboard events, send to app state
- [ ] App state enum: `ServerList | AddServer | EndpointList | RequestBuilder | ResponseViewer`
- [ ] Global key handling: `q` to quit, `Esc` to go back one screen

### Phase 5 — ServerList Screen
- [ ] Render list of servers with `ratatui` List widget
- [ ] Highlight selected item
- [ ] Show loading spinner while fetching spec
- [ ] `a` key opens AddServer modal

### Phase 6 — AddServer Modal
- [ ] Two-field form (name, URL) rendered as a centered popup
- [ ] Text input with cursor via `tui-input` or manual implementation
- [ ] On confirm: add to config, save, trigger spec fetch

### Phase 7 — EndpointList Screen
- [ ] Render all `paths` from parsed spec
- [ ] Color-code HTTP method badges (GET=green, POST=yellow, PUT=blue, DELETE=red, PATCH=cyan)
- [ ] `/` opens an inline filter input; filters list live
- [ ] Enter on an endpoint transitions to RequestBuilder

### Phase 8 — RequestBuilder Screen
- [ ] Read `parameters` (path + query) and `requestBody` schema from selected operation
- [ ] Generate example values using `oaitui-openapi::generate_example`
- [ ] Display as editable table: field name | type | value
- [ ] `e` on a row opens an inline text editor for the value
- [ ] For JSON body: show pretty-printed multi-line editor
- [ ] `Enter` sends request via `oaitui-client`, transitions to ResponseViewer

### Phase 9 — ResponseViewer Screen
- [ ] Show status code with colour (green 2xx, yellow 3xx, red 4xx/5xx)
- [ ] Show elapsed time
- [ ] Show response headers in a collapsible block
- [ ] Pretty-print JSON body with syntax highlighting using `syntect` or simple coloring

### Phase 10 — Polish
- [ ] Persistent history of sent requests (in-memory, per session)
- [ ] Help bar at bottom showing context-sensitive key bindings
- [ ] Error popups for network failures, parse failures
- [ ] `--config` CLI flag to specify config path
- [ ] `--help` and `--version`

---

## Key Dependencies

| Crate | Purpose |
|---|---|
| `ratatui` | TUI framework (widgets, layouts, rendering) |
| `crossterm` | Terminal backend (raw mode, events) |
| `tokio` | Async runtime |
| `reqwest` | HTTP client (async, TLS) |
| `openapiv3` | OpenAPI 3.x spec deserialization |
| `serde` / `serde_json` | JSON serialization |
| `toml` | Config file format |
| `dirs` | XDG config directory resolution |
| `tui-input` | Text input widget for ratatui |
| `syntect` | Syntax highlighting for JSON responses (Phase 10) |
| `anyhow` | Error handling |

---

## Config File Format (`~/.config/oaitui/config.toml`)

```toml
[[servers]]
name = "Petstore"
url  = "https://petstore3.swagger.io/api/v3/openapi.json"
description = "Swagger Petstore example"

[[servers]]
name = "My API"
url  = "https://my-service.example.com/openapi.json"

[servers.default_headers]
Authorization = "Bearer <token>"
```

---

## Architecture Notes

- **No shared mutable state across threads**: the app state lives on the main thread;
  async tasks communicate results back via `tokio::sync::mpsc` channels.
- **Spec caching**: fetched specs are stored in memory keyed by server URL; a manual
  `r` refresh re-fetches.
- **Schema `$ref` resolution**: the `openapiv3` crate resolves `$ref` internally via
  its `ReferenceOr` type; we dereference these when generating examples and building
  the request form.
- **Error surface**: all screens show a dismissable error banner at the top if the
  last async operation failed (e.g. unreachable URL, invalid JSON, HTTP error).
