use oapitui_config::ThemeConfig;
use ratatui::style::Color;

/// Resolved color theme used throughout all UI rendering.
///
/// Every field has a built-in default matching the original hardcoded colors so
/// the application looks identical when no `[theme]` section is present in the
/// config.
#[derive(Debug, Clone)]
pub struct Theme {
    // HTTP method badge colors
    pub method_get: Color,
    pub method_post: Color,
    pub method_put: Color,
    pub method_delete: Color,
    pub method_patch: Color,
    pub method_other: Color,

    // HTTP status-code range colors
    pub status_2xx: Color,
    pub status_3xx: Color,
    pub status_4xx: Color,
    pub status_5xx: Color,
    pub status_other: Color,

    // UI chrome
    pub title: Color,
    pub selected_bg: Color,
    pub border_focused: Color,
    pub border_unfocused: Color,
    pub border_active: Color,
    pub border_editing: Color,

    // Text roles
    pub text_primary: Color,
    pub text_secondary: Color,
    pub text_url: Color,
    pub text_key: Color,
    pub text_tag: Color,
    pub text_accent: Color,

    // Status indicators
    pub indicator_loading: Color,
    pub indicator_success: Color,
    pub indicator_error: Color,

    // Help bar
    pub help_key: Color,
    pub help_desc: Color,

    // Error banner
    pub error: Color,

    // JSON syntax highlighting
    pub json_string: Color,
    pub json_number: Color,
    pub json_bool: Color,
    pub json_null: Color,

    // Markdown rendering
    pub md_h1: Color,
    pub md_h2: Color,
    pub md_code: Color,
    pub md_quote: Color,

    // Parameter list
    pub param_required: Color,
    pub param_location: Color,
    pub param_type: Color,
    pub param_example: Color,

    // Body editor cursors
    pub cursor_block_fg: Color,
    pub cursor_block_bg: Color,
    pub cursor_bar: Color,

    // Endpoint filter bar
    pub filter_active: Color,
    pub filter_inactive: Color,
}

impl Theme {
    /// Build a resolved `Theme` from the optional config overrides, falling
    /// back to the built-in defaults for any field that is not set.
    pub fn from_config(cfg: &ThemeConfig) -> Self {
        fn c(opt: &Option<String>, default: Color) -> Color {
            opt.as_deref().and_then(parse_color).unwrap_or(default)
        }

        Self {
            method_get: c(&cfg.method_get, Color::Green),
            method_post: c(&cfg.method_post, Color::Yellow),
            method_put: c(&cfg.method_put, Color::Blue),
            method_delete: c(&cfg.method_delete, Color::Red),
            method_patch: c(&cfg.method_patch, Color::Cyan),
            method_other: c(&cfg.method_other, Color::White),

            status_2xx: c(&cfg.status_2xx, Color::Green),
            status_3xx: c(&cfg.status_3xx, Color::Yellow),
            status_4xx: c(&cfg.status_4xx, Color::Red),
            status_5xx: c(&cfg.status_5xx, Color::Magenta),
            status_other: c(&cfg.status_other, Color::White),

            title: c(&cfg.title, Color::Cyan),
            selected_bg: c(&cfg.selected_bg, Color::DarkGray),
            border_focused: c(&cfg.border_focused, Color::Cyan),
            border_unfocused: c(&cfg.border_unfocused, Color::DarkGray),
            border_active: c(&cfg.border_active, Color::Yellow),
            border_editing: c(&cfg.border_editing, Color::Green),

            text_primary: c(&cfg.text_primary, Color::White),
            text_secondary: c(&cfg.text_secondary, Color::DarkGray),
            text_url: c(&cfg.text_url, Color::Blue),
            text_key: c(&cfg.text_key, Color::Cyan),
            text_tag: c(&cfg.text_tag, Color::Magenta),
            text_accent: c(&cfg.text_accent, Color::Yellow),

            indicator_loading: c(&cfg.indicator_loading, Color::Yellow),
            indicator_success: c(&cfg.indicator_success, Color::Green),
            indicator_error: c(&cfg.indicator_error, Color::Red),

            help_key: c(&cfg.help_key, Color::Yellow),
            help_desc: c(&cfg.help_desc, Color::DarkGray),

            error: c(&cfg.error, Color::Red),

            json_string: c(&cfg.json_string, Color::Green),
            json_number: c(&cfg.json_number, Color::Yellow),
            json_bool: c(&cfg.json_bool, Color::Magenta),
            json_null: c(&cfg.json_null, Color::DarkGray),

            md_h1: c(&cfg.md_h1, Color::Yellow),
            md_h2: c(&cfg.md_h2, Color::Cyan),
            md_code: c(&cfg.md_code, Color::Green),
            md_quote: c(&cfg.md_quote, Color::DarkGray),

            param_required: c(&cfg.param_required, Color::Red),
            param_location: c(&cfg.param_location, Color::Blue),
            param_type: c(&cfg.param_type, Color::Green),
            param_example: c(&cfg.param_example, Color::Cyan),

            cursor_block_fg: c(&cfg.cursor_block_fg, Color::Black),
            cursor_block_bg: c(&cfg.cursor_block_bg, Color::White),
            cursor_bar: c(&cfg.cursor_bar, Color::Green),

            filter_active: c(&cfg.filter_active, Color::Yellow),
            filter_inactive: c(&cfg.filter_inactive, Color::Cyan),
        }
    }

    /// Map an HTTP method string to its configured badge color.
    pub fn method_color(&self, method: &str) -> Color {
        match method {
            "GET" => self.method_get,
            "POST" => self.method_post,
            "PUT" => self.method_put,
            "DELETE" => self.method_delete,
            "PATCH" => self.method_patch,
            _ => self.method_other,
        }
    }

    /// Map an HTTP status code to its configured range color.
    pub fn status_color(&self, status: u16) -> Color {
        match status {
            200..=299 => self.status_2xx,
            300..=399 => self.status_3xx,
            400..=499 => self.status_4xx,
            500..=599 => self.status_5xx,
            _ => self.status_other,
        }
    }
}

/// Parse a color string into a `ratatui::style::Color`.
///
/// Accepted formats:
/// - Named colors: `"black"`, `"red"`, `"green"`, `"yellow"`, `"blue"`,
///   `"magenta"`, `"cyan"`, `"gray"`, `"dark_gray"`, `"white"`,
///   `"light_red"`, `"light_green"`, `"light_yellow"`, `"light_blue"`,
///   `"light_magenta"`, `"light_cyan"`, `"reset"`
/// - Hex RGB: `"#rrggbb"` (e.g. `"#1e1e2e"`)
/// - ANSI 256-color index: `"42"` (decimal 0–255)
pub fn parse_color(s: &str) -> Option<Color> {
    let s = s.trim().to_ascii_lowercase();

    if let Some(hex) = s.strip_prefix('#') {
        if hex.len() == 6 {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            return Some(Color::Rgb(r, g, b));
        }
        return None;
    }

    match s.as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" | "purple" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "gray" | "grey" => Some(Color::Gray),
        "dark_gray" | "darkgray" | "dark_grey" | "darkgrey" => Some(Color::DarkGray),
        "light_red" | "lightred" => Some(Color::LightRed),
        "light_green" | "lightgreen" => Some(Color::LightGreen),
        "light_yellow" | "lightyellow" => Some(Color::LightYellow),
        "light_blue" | "lightblue" => Some(Color::LightBlue),
        "light_magenta" | "lightmagenta" => Some(Color::LightMagenta),
        "light_cyan" | "lightcyan" => Some(Color::LightCyan),
        "white" => Some(Color::White),
        "reset" => Some(Color::Reset),
        _ => s.parse::<u8>().ok().map(Color::Indexed),
    }
}
