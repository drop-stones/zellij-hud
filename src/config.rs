use std::collections::{BTreeMap, HashMap};

use zellij_tile::prelude::{InputMode, PaletteColor, Styling};

/// RGB or 8-bit terminal color, used throughout the HUD for fg and bg rendering.
#[derive(Clone, Default)]
pub(crate) enum Color {
    #[default]
    None,
    Rgb(u8, u8, u8),
    EightBit(u8),
}

impl Color {
    /// ANSI foreground escape sequence.
    pub(crate) fn fg(&self) -> String {
        match self {
            Color::None => String::new(),
            Color::Rgb(r, g, b) => format!("\x1b[38;2;{};{};{}m", r, g, b),
            Color::EightBit(n) => format!("\x1b[38;5;{}m", n),
        }
    }
    /// ANSI background escape sequence.
    pub(crate) fn bg(&self) -> String {
        match self {
            Color::None => String::new(),
            Color::Rgb(r, g, b) => format!("\x1b[48;2;{};{};{}m", r, g, b),
            Color::EightBit(n) => format!("\x1b[48;5;{}m", n),
        }
    }
    /// Parse from `#RRGGBB` hex or `8bit:N` string.
    fn from_hex(hex: &str) -> Option<Self> {
        if let Some(n) = hex.strip_prefix("8bit:") {
            return Some(Color::EightBit(n.parse().ok()?));
        }
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        if hex.len() != 6 { return None; }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Color::Rgb(r, g, b))
    }
}

/// Rendering style for the HUD status bar.
#[derive(Clone, Copy, PartialEq, Default)]
pub(crate) enum BarStyle {
    /// Flat separators with a single background color (default).
    #[default]
    Minimal,
    /// Each segment has its own background color with Powerline arrow separators.
    Powerline,
}

/// 10-color palette used to derive all UI colors.
pub(crate) struct ThemePalette {
    pub(crate) fg: String,
    pub(crate) bg: String,
    pub(crate) dim: String,
    pub(crate) red: String,
    pub(crate) green: String,
    pub(crate) yellow: String,
    pub(crate) blue: String,
    pub(crate) magenta: String,
    pub(crate) cyan: String,
    pub(crate) orange: String,
}

impl ThemePalette {
    /// Build a palette from zellij's `Styling` (system theme).
    ///
    /// Maps `StyleDeclaration` fields back to semantic palette colors using the
    /// known `Palette → Styling` conversion in zellij (data.rs:1577).
    pub(crate) fn from_styling(s: &Styling) -> Self {
        let fg = s.text_unselected.base;
        // Derive dim from fg. table_title.background maps to palette.gray, but
        // old-style palette themes don't define gray and new-style themes may
        // assign arbitrary colors. A dimmed fg is consistently readable.
        let dim = dim_color(fg);
        Self {
            fg: palette_color_to_hex(fg),
            bg: palette_color_to_hex(s.ribbon_unselected.base),
            dim: palette_color_to_hex(dim),
            red: palette_color_to_hex(s.exit_code_error.base),
            green: palette_color_to_hex(s.exit_code_success.base),
            yellow: palette_color_to_hex(s.exit_code_error.emphasis_0),
            blue: palette_color_to_hex(s.ribbon_unselected.emphasis_2),
            magenta: palette_color_to_hex(s.text_unselected.emphasis_3),
            cyan: palette_color_to_hex(s.text_unselected.emphasis_1),
            orange: palette_color_to_hex(s.text_unselected.emphasis_0),
        }
    }

    /// Look up a built-in theme by name. Unknown names fall back to tokyonight.
    pub(crate) fn from_name(name: &str) -> Self {
        match name {
            "catppuccin-mocha" => Self {
                fg: "#cdd6f4".into(),
                bg: "#1e1e2e".into(),
                dim: "#585b70".into(),
                red: "#f38ba8".into(),
                green: "#a6e3a1".into(),
                yellow: "#f9e2af".into(),
                blue: "#89b4fa".into(),
                magenta: "#cba6f7".into(),
                cyan: "#89dceb".into(),
                orange: "#fab387".into(),
            },
            "nord" => Self {
                fg: "#eceff4".into(),
                bg: "#2e3440".into(),
                dim: "#4c566a".into(),
                red: "#bf616a".into(),
                green: "#a3be8c".into(),
                yellow: "#ebcb8b".into(),
                blue: "#81a1c1".into(),
                magenta: "#b48ead".into(),
                cyan: "#88c0d0".into(),
                orange: "#d08770".into(),
            },
            "gruvbox-dark" => Self {
                fg: "#ebdbb2".into(),
                bg: "#282828".into(),
                dim: "#665c54".into(),
                red: "#fb4934".into(),
                green: "#b8bb26".into(),
                yellow: "#fabd2f".into(),
                blue: "#83a598".into(),
                magenta: "#d3869b".into(),
                cyan: "#8ec07c".into(),
                orange: "#fe8019".into(),
            },
            // tokyonight (default)
            _ => Self::default(),
        }
    }

    /// Apply `palette_*` overrides from user config.
    pub(crate) fn apply_overrides(&mut self, config: &BTreeMap<String, String>) {
        macro_rules! override_field {
            ($key:expr, $field:expr) => {
                if let Some(v) = config.get($key) {
                    $field = v.clone();
                }
            };
        }
        override_field!("palette_fg", self.fg);
        override_field!("palette_bg", self.bg);
        override_field!("palette_dim", self.dim);
        override_field!("palette_red", self.red);
        override_field!("palette_green", self.green);
        override_field!("palette_yellow", self.yellow);
        override_field!("palette_blue", self.blue);
        override_field!("palette_magenta", self.magenta);
        override_field!("palette_cyan", self.cyan);
        override_field!("palette_orange", self.orange);
    }
}

impl ThemePalette {
    /// Resolve a palette color name to its hex value.
    pub(crate) fn resolve(&self, name: &str) -> Option<&str> {
        match name {
            "fg" => Some(&self.fg),
            "bg" => Some(&self.bg),
            "dim" => Some(&self.dim),
            "red" => Some(&self.red),
            "green" => Some(&self.green),
            "yellow" => Some(&self.yellow),
            "blue" => Some(&self.blue),
            "magenta" => Some(&self.magenta),
            "cyan" => Some(&self.cyan),
            "orange" => Some(&self.orange),
            _ => None,
        }
    }
}

/// Produce a dimmed variant of a color (50% brightness).
fn dim_color(color: PaletteColor) -> PaletteColor {
    match color {
        PaletteColor::Rgb((r, g, b)) => PaletteColor::Rgb((r / 2, g / 2, b / 2)),
        // EightBit(8) = "bright black" = dark gray in most terminals
        PaletteColor::EightBit(_) => PaletteColor::EightBit(8),
    }
}

/// Convert a `PaletteColor` to a hex string usable by `Color::from_hex`.
/// Rgb → "#rrggbb", EightBit → "8bit:N".
fn palette_color_to_hex(color: PaletteColor) -> String {
    match color {
        PaletteColor::Rgb((r, g, b)) => format!("#{:02x}{:02x}{:02x}", r, g, b),
        PaletteColor::EightBit(n) => format!("8bit:{}", n),
    }
}

impl Default for ThemePalette {
    fn default() -> Self {
        Self {
            fg: "#c0caf5".into(),
            bg: "#1a1b26".into(),
            dim: "#565f89".into(),
            red: "#f7768e".into(),
            green: "#9ece6a".into(),
            yellow: "#e0af68".into(),
            blue: "#7aa2f7".into(),
            magenta: "#bb9af7".into(),
            cyan: "#2ac3de".into(),
            orange: "#ff9e64".into(),
        }
    }
}

/// ANSI color escapes for icon categories in the tooltip.
pub(crate) struct IconColors {
    pub(crate) navigation: Color,
    pub(crate) create: Color,
    pub(crate) close: Color,
    pub(crate) resize: Color,
    pub(crate) toggle: Color,
    pub(crate) search: Color,
    pub(crate) mode_switch: Color,
    pub(crate) plugin: Color,
    pub(crate) dim: Color,
}

impl IconColors {
    fn from_palette(p: &ThemePalette) -> Self {
        let c = |hex: &str| Color::from_hex(hex).unwrap_or_default();
        Self {
            navigation: c(&p.cyan),
            create: c(&p.green),
            close: c(&p.red),
            resize: c(&p.orange),
            toggle: c(&p.yellow),
            search: c(&p.magenta),
            mode_switch: c(&p.blue),
            plugin: c(&p.fg),
            dim: c(&p.dim),
        }
    }
}

pub(crate) struct HudConfig {
    pub(crate) format_left: String,
    pub(crate) format_right: String,
    pub(crate) color_bg: Color,
    pub(crate) color_session: Color,
    pub(crate) color_mode: Color,
    pub(crate) mode_colors: HashMap<InputMode, Color>,
    pub(crate) color_tab_active: Color,
    pub(crate) color_tab_inactive: Color,
    pub(crate) color_cwd: Color,
    pub(crate) color_date: Color,
    pub(crate) color_time: Color,
    pub(crate) color_memory: Color,
    pub(crate) color_separator: Color,
    pub(crate) color_tooltip_key: Color,
    pub(crate) color_tooltip_arrow: Color,
    pub(crate) color_tooltip_action: Color,
    pub(crate) color_tooltip_mode: Color,
    pub(crate) icon_colors: IconColors,
    pub(crate) enable_status_bar: bool,
    pub(crate) enable_tooltip: bool,
    pub(crate) separator: String,
    /// Powerline mode: each segment gets its own background color with arrow separators.
    /// HUD rendering style (minimal or powerline).
    pub(crate) bar: BarStyle,
    /// Separator arrow for the left area (powerline mode only).
    pub(crate) separator_left: String,
    /// Separator arrow for the right area (powerline mode only).
    pub(crate) separator_right: String,
    pub(crate) timezone_offset: i64,
    /// Whether to use zellij's theme colors (theme "system").
    pub(crate) use_system_theme: bool,
}

impl HudConfig {
    pub(crate) fn from_config(config: &BTreeMap<String, String>) -> Self {
        let use_system_theme = config.get("theme").map_or(true, |t| t == "system");

        // For "system" (default), use tokyonight as placeholder until ModeUpdate delivers Styling.
        let mut palette = match config.get("theme") {
            Some(name) if name != "system" => ThemePalette::from_name(name),
            _ => ThemePalette::default(),
        };
        palette.apply_overrides(config);

        let mut hud = Self::build_from_palette(&palette, config);
        hud.use_system_theme = use_system_theme;
        hud
    }

    /// Rebuild colors from zellij's system theme. Called when ModeUpdate arrives.
    pub(crate) fn apply_system_theme(
        &mut self,
        styling: &Styling,
        config: &BTreeMap<String, String>,
    ) {
        let mut palette = ThemePalette::from_styling(styling);
        palette.apply_overrides(config);
        let rebuilt = Self::build_from_palette(&palette, config);

        // Update color fields only; preserve non-color config (format, separator, etc.)
        self.color_bg = rebuilt.color_bg;
        self.color_session = rebuilt.color_session;
        self.color_mode = rebuilt.color_mode;
        self.mode_colors = rebuilt.mode_colors;
        self.color_tab_active = rebuilt.color_tab_active;
        self.color_tab_inactive = rebuilt.color_tab_inactive;
        self.color_cwd = rebuilt.color_cwd;
        self.color_date = rebuilt.color_date;
        self.color_time = rebuilt.color_time;
        self.color_memory = rebuilt.color_memory;
        self.color_separator = rebuilt.color_separator;
        self.color_tooltip_key = rebuilt.color_tooltip_key;
        self.color_tooltip_arrow = rebuilt.color_tooltip_arrow;
        self.color_tooltip_action = rebuilt.color_tooltip_action;
        self.color_tooltip_mode = rebuilt.color_tooltip_mode;
        self.icon_colors = rebuilt.icon_colors;
    }

    fn build_from_palette(palette: &ThemePalette, config: &BTreeMap<String, String>) -> Self {
        let color = |hex: &str| Color::from_hex(hex).unwrap_or_default();

        let mode_colors = HashMap::from([
            (InputMode::Normal, color(&palette.green)),
            (InputMode::Locked, color(&palette.dim)),
            (InputMode::Pane, color(&palette.orange)),
            (InputMode::Tab, color(&palette.yellow)),
            (InputMode::Resize, color(&palette.red)),
            (InputMode::Move, color(&palette.magenta)),
            (InputMode::Scroll, color(&palette.cyan)),
            (InputMode::Session, color(&palette.magenta)),
            (InputMode::Search, color(&palette.yellow)),
            (InputMode::RenameTab, color(&palette.yellow)),
            (InputMode::RenamePane, color(&palette.yellow)),
            (InputMode::EnterSearch, color(&palette.yellow)),
            (InputMode::Tmux, color(&palette.orange)),
            (InputMode::Prompt, color(&palette.blue)),
        ]);

        let icon_colors = IconColors::from_palette(palette);

        let mut hud = Self {
            format_left: "{session} | {mode} | {tabs}".to_string(),
            format_right: "{cwd} | {memory} | {date} | {time}".to_string(),
            color_bg: color(&palette.bg),
            color_session: color(&palette.cyan),
            color_mode: color(&palette.blue),
            mode_colors,
            color_tab_active: color(&palette.fg),
            color_tab_inactive: color(&palette.dim),
            color_cwd: color(&palette.cyan),
            color_date: color(&palette.magenta),
            color_time: color(&palette.blue),
            color_memory: color(&palette.green),
            color_separator: color(&palette.dim),
            color_tooltip_key: color(&palette.cyan),
            color_tooltip_arrow: color(&palette.dim),
            color_tooltip_action: color(&palette.magenta),
            color_tooltip_mode: color(&palette.blue),
            icon_colors,
            enable_status_bar: true,
            enable_tooltip: true,
            separator: "│".to_string(),
            bar: BarStyle::Minimal,
            separator_left: "\u{e0b0}".to_string(),
            separator_right: "\u{e0b2}".to_string(),
            timezone_offset: 0,
            use_system_theme: false,
        };

        // Apply color_* overrides (hex or palette name)
        macro_rules! color_override {
            ($key:expr, $field:expr) => {
                if let Some(v) = config.get($key) {
                    if let Some(c) = Self::resolve_color(v, palette) {
                        $field = c;
                    }
                }
            };
        }
        color_override!("color_bg", hud.color_bg);
        color_override!("color_session", hud.color_session);
        color_override!("color_mode", hud.color_mode);
        color_override!("color_tab_active", hud.color_tab_active);
        color_override!("color_tab_inactive", hud.color_tab_inactive);
        color_override!("color_cwd", hud.color_cwd);
        color_override!("color_date", hud.color_date);
        color_override!("color_time", hud.color_time);
        color_override!("color_memory", hud.color_memory);
        color_override!("color_separator", hud.color_separator);
        color_override!("color_tooltip_key", hud.color_tooltip_key);
        color_override!("color_tooltip_arrow", hud.color_tooltip_arrow);
        color_override!("color_tooltip_action", hud.color_tooltip_action);
        color_override!("color_tooltip_mode", hud.color_tooltip_mode);

        // color_mode_* overrides
        let mode_map = [
            ("color_mode_normal", InputMode::Normal),
            ("color_mode_locked", InputMode::Locked),
            ("color_mode_pane", InputMode::Pane),
            ("color_mode_tab", InputMode::Tab),
            ("color_mode_resize", InputMode::Resize),
            ("color_mode_move", InputMode::Move),
            ("color_mode_scroll", InputMode::Scroll),
            ("color_mode_session", InputMode::Session),
            ("color_mode_search", InputMode::Search),
            ("color_mode_rename_tab", InputMode::RenameTab),
            ("color_mode_rename_pane", InputMode::RenamePane),
            ("color_mode_enter_search", InputMode::EnterSearch),
            ("color_mode_tmux", InputMode::Tmux),
            ("color_mode_prompt", InputMode::Prompt),
        ];
        for (key, mode) in &mode_map {
            if let Some(v) = config.get(*key) {
                if let Some(c) = Self::resolve_color(v, palette) {
                    hud.mode_colors.insert(*mode, c);
                }
            }
        }

        if let Some(v) = config.get("format_left") {
            hud.format_left = v.clone();
        }
        if let Some(v) = config.get("format_right") {
            hud.format_right = v.clone();
        }
        if let Some(v) = config.get("separator") {
            hud.separator = v.clone();
        }
        if let Some(v) = config.get("bar") {
            hud.bar = match v.as_str() {
                "powerline" => BarStyle::Powerline,
                _ => BarStyle::Minimal,
            };
        }
        // separator_left/right: if only left is set, right falls back to left's value.
        match (config.get("separator_left"), config.get("separator_right")) {
            (Some(l), Some(r)) => {
                hud.separator_left = l.clone();
                hud.separator_right = r.clone();
            }
            (Some(l), None) => {
                hud.separator_left = l.clone();
                hud.separator_right = l.clone();
            }
            (None, Some(r)) => {
                hud.separator_right = r.clone();
            }
            (None, None) => {}
        }
        if let Some(v) = config.get("enable_status_bar") {
            hud.enable_status_bar = v != "false";
        }
        if let Some(v) = config.get("enable_tooltip") {
            hud.enable_tooltip = v != "false";
        }

        hud
    }

    pub(crate) fn color_for_mode(&self, mode: InputMode) -> &Color {
        self.mode_colors.get(&mode).unwrap_or(&self.color_mode)
    }

    /// Resolve a palette name or hex string into a `Color`.
    fn resolve_color(value: &str, palette: &ThemePalette) -> Option<Color> {
        let hex = palette.resolve(value).unwrap_or(value);
        Color::from_hex(hex)
    }
}

impl Default for HudConfig {
    fn default() -> Self {
        Self::from_config(&BTreeMap::new())
    }
}
