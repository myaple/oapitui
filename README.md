# oaitui

A terminal UI for exploring and testing OpenAPI services.

Load any OpenAPI 3.x spec by URL, browse endpoints, fill in parameters, fire requests, and inspect responses — all without leaving the terminal.

---

## Installation

**From a release binary**

Download the latest binary from the [Releases](../../releases) page and put it on your `$PATH`.

**From source**

```sh
cargo install --path crates/tui
```

**Docker**

```sh
docker run -it --rm ghcr.io/your-org/oaitui
```

---

## Configuration

oaitui stores servers in a TOML config file.

Default location: `~/.config/oaitui/config.toml`

Custom location: `oaitui --config /path/to/config.toml`

```toml
[[servers]]
name        = "Petstore"
url         = "https://raw.githubusercontent.com/readmeio/oas-examples/main/3.1/json/petstore.json"
description = "OpenAPI Petstore example"

[[servers]]
name        = "My API"
url         = "https://api.example.com/openapi.json"
description = "Internal service"

[servers.default_headers]
Authorization = "Bearer your-token-here"
```

Servers are saved automatically when added through the TUI.

---

## Usage

```sh
oaitui
```

### Server list

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate servers |
| `Enter` | Open server |
| `a` | Add server |
| `d` | Delete server |
| `r` | Refresh spec |
| `Ctrl+C` | Quit |

### Endpoint list

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate endpoints |
| `/` | Filter by method, path, or summary |
| `Enter` | Open endpoint |
| `Esc` | Back |

### Request builder

The request builder has two panes: a **params table** (top) and a **body editor** (bottom).

**Params pane** (focused by default)

| Key | Action |
|-----|--------|
| `j` / `k` | Move between parameters |
| `e` | Edit selected value |
| `Esc` | Stop editing / go back |
| `Tab` | Focus body pane |
| `Enter` | Send request |

**Body pane — normal mode** (`Tab` to focus)

| Key | Action |
|-----|--------|
| `h` `j` `k` `l` | Move cursor |
| `0` / `$` | Start / end of line |
| `gg` / `G` | Top / bottom of body |
| `i` / `a` | Insert before / after cursor |
| `I` / `A` | Insert at line start / end |
| `dd` | Delete current line |
| `dw` | Delete word |
| `Esc` / `Tab` | Unfocus, return to params |

**Body pane — insert mode** (`i` / `a` to enter)

| Key | Action |
|-----|--------|
| `Esc` | Back to normal mode |

### Response viewer

| Key | Action |
|-----|--------|
| `j` / `k` | Scroll |
| `h` | Toggle response headers |
| `Esc` | Back |

---

## Notes

- Specs are fetched from the URL you provide. GitHub blob URLs (`/blob/`) won't work — use the raw URL (`/raw/` or `raw.githubusercontent.com`).
- Both JSON and YAML specs are supported.
- `$ref` resolution is handled inline for parameters, request bodies, and schemas.
