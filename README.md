# oapitui

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
docker run -it --rm ghcr.io/your-org/oapitui
```

---

## Configuration

oapitui stores servers in a TOML config file.

Default location: `~/.config/oapitui/config.toml`

Custom location: `oapitui --config /path/to/config.toml`

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

**Mutual TLS (mTLS)** — add a `[servers.tls]` block per server:

```toml
[[servers]]
name = "Secure API"
url  = "https://api.internal/openapi.json"

[servers.tls]
client_cert = "/path/to/client.crt"   # PEM client certificate
client_key  = "/path/to/client.key"   # PEM client private key
ca_cert     = "/path/to/ca.crt"       # Custom CA for server verification (optional)
```

All three paths are optional — omit `ca_cert` to use system roots; omit the cert/key pair if the server doesn't require client auth. TLS settings are applied to both spec fetching and API requests.

Servers are saved automatically when added through the TUI. mTLS paths entered in the Add Server form are persisted to the config file.

### Environments

Define named environments with variables that are substituted into parameter values, headers, and URLs before sending requests. Use `{{variable_name}}` syntax in any parameter value.

```toml
[[environments]]
name = "dev"
[environments.variables]
base_url = "http://localhost:3000"
api_key  = "dev-key-123"
token    = "Bearer dev-token"

[[environments]]
name = "staging"
[environments.variables]
base_url = "https://staging.example.com"
api_key  = "staging-key-456"
token    = "Bearer staging-token"

[[environments]]
name = "prod"
[environments.variables]
base_url = "https://api.example.com"
api_key  = "prod-key-secret"
token    = "Bearer prod-token"
```

Press `E` from the server list, endpoint list, or request builder to switch environments. The active environment is shown in the bottom-right of the help bar.

### Request History

Every successful request is automatically saved to `~/.config/oapitui/history.json` (up to 200 entries). Press `H` from the server list to browse history, filter by method/path/server, and re-open past requests with their saved parameter values.

---

## Usage

```sh
oapitui
```

### Server list

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate servers |
| `Enter` | Open server |
| `a` | Add server |
| `d` | Delete server |
| `r` | Refresh spec |
| `H` | Open request history |
| `E` | Switch environment |
| `Ctrl+C` | Quit |

### Endpoint list

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate endpoints |
| `/` | Filter by method, path, or summary |
| `E` | Switch environment |
| `Enter` | Open endpoint |
| `Esc` | Back |

### Request builder

The request builder has two panes: a **params table** (top) and a **body editor** (bottom).

**Params pane** (focused by default)

| Key | Action |
|-----|--------|
| `j` / `k` | Move between parameters |
| `e` | Edit selected value |
| `E` | Switch environment |
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

### History

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate entries |
| `PgUp` / `PgDn` | Page through entries |
| `/` | Filter by method, path, or server |
| `Enter` | Re-open endpoint with saved params |
| `d` | Delete selected entry |
| `Esc` | Back to server list |

---

## Themes

Every color in the TUI can be overridden via a `[theme]` section in your config file. All fields are optional — omit any to keep the built-in default.

```toml
[theme]
# HTTP method badge colors
method_get    = "green"
method_post   = "yellow"
method_put    = "blue"
method_delete = "red"
method_patch  = "cyan"
method_other  = "white"

# HTTP status-code range colors
status_2xx   = "green"
status_3xx   = "yellow"
status_4xx   = "red"
status_5xx   = "magenta"
status_other = "white"

# UI chrome
title            = "cyan"      # block/section title text
selected_bg      = "dark_gray" # selected list-item background
border_focused   = "cyan"      # active pane border
border_unfocused = "dark_gray" # inactive pane border
border_active    = "yellow"    # focused editable block / add-server active field
border_editing   = "green"     # body editor in INSERT mode

# Text roles
text_primary   = "white"     # main content text
text_secondary = "dark_gray" # labels, hints, secondary info
text_url       = "blue"      # server URLs
text_key       = "cyan"      # JSON keys, header names, parameter names
text_tag       = "magenta"   # endpoint tags
text_accent    = "yellow"    # operation IDs, content-type values, table column headers

# Status indicators (icons next to server entries)
indicator_loading = "yellow"
indicator_success = "green"
indicator_error   = "red"

# Help bar (bottom of screen)
help_key  = "yellow"    # keybinding labels  e.g. "Enter"
help_desc = "dark_gray" # keybinding descriptions e.g. "open"

# Error banner
error = "red"

# JSON response syntax highlighting
json_string = "green"
json_number = "yellow"
json_bool   = "magenta"
json_null   = "dark_gray"

# Markdown rendering (server description panel)
md_h1    = "yellow"
md_h2    = "cyan"
md_code  = "green"
md_quote = "dark_gray"

# Parameter list
param_required = "red"   # * required marker
param_location = "blue"  # [path] / [query] / [header]
param_type     = "green" # type label
param_example  = "cyan"  # example value

# Body editor cursors
cursor_block_fg = "black" # block cursor foreground (normal mode)
cursor_block_bg = "white" # block cursor background (normal mode)
cursor_bar      = "green" # bar cursor (insert mode)

# Endpoint filter bar
filter_active   = "yellow" # while actively typing a filter
filter_inactive = "cyan"   # filter shown but not being edited
```

### Color values

| Format | Example | Description |
|--------|---------|-------------|
| Named | `"cyan"` | `black`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `gray`, `dark_gray`, `white`, `light_red`, `light_green`, `light_yellow`, `light_blue`, `light_magenta`, `light_cyan`, `reset` |
| Hex RGB | `"#1e1e2e"` | 24-bit `#rrggbb` — requires a true-color terminal |
| ANSI index | `"42"` | 256-color palette index `0`–`255` |

### Example: Catppuccin Mocha

```toml
[theme]
method_get    = "#a6e3a1"
method_post   = "#f9e2af"
method_put    = "#89b4fa"
method_delete = "#f38ba8"
method_patch  = "#89dceb"

status_2xx = "#a6e3a1"
status_3xx = "#f9e2af"
status_4xx = "#f38ba8"
status_5xx = "#cba6f7"

title          = "#89dceb"
selected_bg    = "#313244"
border_focused = "#89dceb"
border_active  = "#f9e2af"
border_editing = "#a6e3a1"

text_primary   = "#cdd6f4"
text_secondary = "#6c7086"
text_url       = "#89b4fa"
text_key       = "#89dceb"
text_tag       = "#cba6f7"
text_accent    = "#f9e2af"

json_string = "#a6e3a1"
json_number = "#f9e2af"
json_bool   = "#cba6f7"
json_null   = "#6c7086"

md_h1   = "#f9e2af"
md_h2   = "#89dceb"
md_code = "#a6e3a1"

help_key  = "#f9e2af"
help_desc = "#6c7086"
error     = "#f38ba8"
```

---

## Notes

- Specs are fetched from the URL you provide. GitHub blob URLs (`/blob/`) won't work — use the raw URL (`/raw/` or `raw.githubusercontent.com`).
- Both JSON and YAML specs are supported.
- `$ref` resolution is handled inline for parameters, request bodies, and schemas.
