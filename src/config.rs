use std::collections::{BTreeMap, HashMap};

use zellij_tile::prelude::InputMode;

/// Which mode is considered the "home" mode where HUD hides.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum BaseMode {
    /// Auto-detect from keybindings.
    Auto,
    /// Lock-centric: HUD hides in Locked mode.
    Locked,
    /// Normal-centric: HUD hides in Normal mode.
    Normal,
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
    pub(crate) navigation: String,
    pub(crate) create: String,
    pub(crate) close: String,
    pub(crate) resize: String,
    pub(crate) toggle: String,
    pub(crate) search: String,
    pub(crate) mode_switch: String,
    pub(crate) dim: String,
}

impl IconColors {
    fn from_palette(p: &ThemePalette) -> Self {
        Self {
            navigation: HudConfig::hex_to_fg(&p.cyan).unwrap_or_default(),
            create: HudConfig::hex_to_fg(&p.green).unwrap_or_default(),
            close: HudConfig::hex_to_fg(&p.red).unwrap_or_default(),
            resize: HudConfig::hex_to_fg(&p.orange).unwrap_or_default(),
            toggle: HudConfig::hex_to_fg(&p.yellow).unwrap_or_default(),
            search: HudConfig::hex_to_fg(&p.magenta).unwrap_or_default(),
            mode_switch: HudConfig::hex_to_fg(&p.blue).unwrap_or_default(),
            dim: HudConfig::hex_to_fg(&p.dim).unwrap_or_default(),
        }
    }
}

pub(crate) struct HudConfig {
    pub(crate) format_left: String,
    pub(crate) format_right: String,
    pub(crate) color_session: String,
    pub(crate) color_mode: String,
    pub(crate) mode_colors: HashMap<InputMode, String>,
    pub(crate) color_tab_active: String,
    pub(crate) color_tab_inactive: String,
    pub(crate) color_cwd: String,
    pub(crate) color_date: String,
    pub(crate) color_time: String,
    pub(crate) color_memory: String,
    pub(crate) color_separator: String,
    pub(crate) color_tooltip_key: String,
    pub(crate) color_tooltip_arrow: String,
    pub(crate) color_tooltip_action: String,
    pub(crate) color_tooltip_mode: String,
    pub(crate) icon_colors: IconColors,
    pub(crate) enable_status_bar: bool,
    pub(crate) enable_tooltip: bool,
    pub(crate) base_mode: BaseMode,
    pub(crate) separator: String,
    pub(crate) timezone_offset: i64,
}

impl HudConfig {
    pub(crate) fn from_config(config: &BTreeMap<String, String>) -> Self {
        // 1. Resolve theme palette
        let mut palette = match config.get("theme") {
            Some(name) => ThemePalette::from_name(name),
            None => ThemePalette::default(),
        };

        // 2. Apply palette_* overrides
        palette.apply_overrides(config);

        // 3. Derive all ANSI defaults from palette
        let fg = |hex: &str| Self::hex_to_fg(hex).unwrap_or_default();

        let mode_colors = HashMap::from([
            (InputMode::Normal, fg(&palette.green)),
            (InputMode::Locked, fg(&palette.dim)),
            (InputMode::Pane, fg(&palette.orange)),
            (InputMode::Tab, fg(&palette.yellow)),
            (InputMode::Resize, fg(&palette.red)),
            (InputMode::Move, fg(&palette.magenta)),
            (InputMode::Scroll, fg(&palette.cyan)),
            (InputMode::Session, fg(&palette.magenta)),
            (InputMode::Search, fg(&palette.yellow)),
            (InputMode::RenameTab, fg(&palette.yellow)),
            (InputMode::RenamePane, fg(&palette.yellow)),
            (InputMode::EnterSearch, fg(&palette.yellow)),
            (InputMode::Tmux, fg(&palette.orange)),
            (InputMode::Prompt, fg(&palette.blue)),
        ]);

        let icon_colors = IconColors::from_palette(&palette);

        let mut hud = Self {
            format_left: "{session} | {mode} | {tabs}".to_string(),
            format_right: "{cwd} | {memory} | {date} | {time}".to_string(),
            color_session: fg(&palette.cyan),
            color_mode: fg(&palette.blue),
            mode_colors,
            color_tab_active: fg(&palette.fg),
            color_tab_inactive: fg(&palette.dim),
            color_cwd: fg(&palette.cyan),
            color_date: fg(&palette.magenta),
            color_time: fg(&palette.blue),
            color_memory: fg(&palette.green),
            color_separator: fg(&palette.dim),
            color_tooltip_key: fg(&palette.cyan),
            color_tooltip_arrow: fg(&palette.dim),
            color_tooltip_action: fg(&palette.magenta),
            color_tooltip_mode: fg(&palette.blue),
            icon_colors,
            enable_status_bar: true,
            enable_tooltip: true,
            base_mode: BaseMode::Auto,
            separator: "│".to_string(),
            timezone_offset: 0,
        };

        // 4. Apply color_* overrides (hex or palette name)
        macro_rules! color_fg {
            ($key:expr, $field:expr) => {
                if let Some(v) = config.get($key) {
                    if let Some(c) = Self::resolve_fg(v, &palette) {
                        $field = c;
                    }
                }
            };
        }
        color_fg!("color_session", hud.color_session);
        color_fg!("color_mode", hud.color_mode);
        color_fg!("color_tab_active", hud.color_tab_active);
        color_fg!("color_tab_inactive", hud.color_tab_inactive);
        color_fg!("color_cwd", hud.color_cwd);
        color_fg!("color_date", hud.color_date);
        color_fg!("color_time", hud.color_time);
        color_fg!("color_memory", hud.color_memory);
        color_fg!("color_separator", hud.color_separator);
        color_fg!("color_tooltip_key", hud.color_tooltip_key);
        color_fg!("color_tooltip_arrow", hud.color_tooltip_arrow);
        color_fg!("color_tooltip_action", hud.color_tooltip_action);
        color_fg!("color_tooltip_mode", hud.color_tooltip_mode);

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
                if let Some(c) = Self::resolve_fg(v, &palette) {
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
        if let Some(v) = config.get("timezone") {
            if let Ok(n) = v.parse::<i64>() {
                hud.timezone_offset = n;
            }
        }
        if let Some(v) = config.get("enable_status_bar") {
            hud.enable_status_bar = v != "false";
        }
        if let Some(v) = config.get("enable_tooltip") {
            hud.enable_tooltip = v != "false";
        }
        if let Some(v) = config.get("base_mode") {
            hud.base_mode = match v.as_str() {
                "locked" => BaseMode::Locked,
                "normal" => BaseMode::Normal,
                _ => BaseMode::Auto,
            };
        }

        hud
    }

    pub(crate) fn color_for_mode(&self, mode: InputMode) -> &str {
        self.mode_colors
            .get(&mode)
            .map_or(&self.color_mode, |c| c.as_str())
    }

    /// Resolve a value as palette name or hex, then convert to fg ANSI.
    fn resolve_fg(value: &str, palette: &ThemePalette) -> Option<String> {
        let hex = palette.resolve(value).unwrap_or(value);
        Self::hex_to_fg(hex)
    }

    pub(crate) fn hex_to_fg(hex: &str) -> Option<String> {
        let (r, g, b) = Self::parse_hex(hex)?;
        Some(format!("\x1b[38;2;{};{};{}m", r, g, b))
    }

    fn parse_hex(hex: &str) -> Option<(u8, u8, u8)> {
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        if hex.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some((r, g, b))
    }
}

impl Default for HudConfig {
    fn default() -> Self {
        Self::from_config(&BTreeMap::new())
    }
}
