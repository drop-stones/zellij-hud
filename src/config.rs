use std::collections::{BTreeMap, HashMap};

use zellij_tile::prelude::InputMode;

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
    pub(crate) color_bg: String,
    pub(crate) color_tooltip_key: String,
    pub(crate) color_tooltip_arrow: String,
    pub(crate) color_tooltip_action: String,
    pub(crate) color_tooltip_mode: String,
    pub(crate) separator: String,
    pub(crate) timezone_offset: i64,
}

impl HudConfig {
    pub(crate) fn from_config(config: &BTreeMap<String, String>) -> Self {
        let mut hud = Self::default();

        if let Some(v) = config.get("format_left") {
            hud.format_left = v.clone();
        }
        if let Some(v) = config.get("format_right") {
            hud.format_right = v.clone();
        }
        if let Some(v) = config.get("color_session") {
            if let Some(c) = Self::hex_to_fg(v) {
                hud.color_session = c;
            }
        }
        if let Some(v) = config.get("color_mode") {
            if let Some(c) = Self::hex_to_fg(v) {
                hud.color_mode = c;
            }
        }
        if let Some(v) = config.get("color_tab_active") {
            if let Some(c) = Self::hex_to_fg(v) {
                hud.color_tab_active = c;
            }
        }
        if let Some(v) = config.get("color_tab_inactive") {
            if let Some(c) = Self::hex_to_fg(v) {
                hud.color_tab_inactive = c;
            }
        }
        if let Some(v) = config.get("color_cwd") {
            if let Some(c) = Self::hex_to_fg(v) {
                hud.color_cwd = c;
            }
        }
        if let Some(v) = config.get("color_date") {
            if let Some(c) = Self::hex_to_fg(v) {
                hud.color_date = c;
            }
        }
        if let Some(v) = config.get("color_time") {
            if let Some(c) = Self::hex_to_fg(v) {
                hud.color_time = c;
            }
        }
        if let Some(v) = config.get("color_memory") {
            if let Some(c) = Self::hex_to_fg(v) {
                hud.color_memory = c;
            }
        }
        if let Some(v) = config.get("color_separator") {
            if let Some(c) = Self::hex_to_fg(v) {
                hud.color_separator = c;
            }
        }
        if let Some(v) = config.get("color_bg") {
            if let Some(c) = Self::hex_to_bg(v) {
                hud.color_bg = c;
            }
        }
        if let Some(v) = config.get("color_tooltip_key") {
            if let Some(c) = Self::hex_to_fg(v) {
                hud.color_tooltip_key = c;
            }
        }
        if let Some(v) = config.get("color_tooltip_arrow") {
            if let Some(c) = Self::hex_to_fg(v) {
                hud.color_tooltip_arrow = c;
            }
        }
        if let Some(v) = config.get("color_tooltip_action") {
            if let Some(c) = Self::hex_to_fg(v) {
                hud.color_tooltip_action = c;
            }
        }
        if let Some(v) = config.get("color_tooltip_mode") {
            if let Some(c) = Self::hex_to_fg(v) {
                hud.color_tooltip_mode = c;
            }
        }
        if let Some(v) = config.get("separator") {
            hud.separator = v.clone();
        }
        if let Some(v) = config.get("timezone") {
            if let Ok(n) = v.parse::<i64>() {
                hud.timezone_offset = n;
            }
        }

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
                if let Some(c) = Self::hex_to_fg(v) {
                    hud.mode_colors.insert(*mode, c);
                }
            }
        }

        hud
    }

    pub(crate) fn color_for_mode(&self, mode: InputMode) -> &str {
        self.mode_colors
            .get(&mode)
            .map_or(&self.color_mode, |c| c.as_str())
    }

    fn hex_to_fg(hex: &str) -> Option<String> {
        let (r, g, b) = Self::parse_hex(hex)?;
        Some(format!("\x1b[38;2;{};{};{}m", r, g, b))
    }

    fn hex_to_bg(hex: &str) -> Option<String> {
        let (r, g, b) = Self::parse_hex(hex)?;
        Some(format!("\x1b[48;2;{};{};{}m", r, g, b))
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
        Self {
            format_left: "{session} | {mode} | {tabs}".to_string(),
            format_right: "{cwd} | {memory} | {date} | {time}".to_string(),
            color_session: "\x1b[38;2;42;195;222m".to_string(), // #2ac3de
            color_mode: "\x1b[38;2;140;165;240m".to_string(),   // #8ca5f0
            mode_colors: HashMap::from([
                (InputMode::Normal, "\x1b[38;2;158;206;106m".to_string()), // #9ece6a green
                (InputMode::Locked, "\x1b[38;2;86;95;137m".to_string()),   // #565f89 dim
                (InputMode::Resize, "\x1b[38;2;247;118;142m".to_string()), // #f7768e red
            ]),
            color_tab_active: "\x1b[38;2;192;202;245m".to_string(), // #c0caf5
            color_tab_inactive: "\x1b[38;2;86;95;137m".to_string(), // #565f89
            color_cwd: "\x1b[38;2;42;195;222m".to_string(),         // #2ac3de
            color_date: "\x1b[38;2;187;154;247m".to_string(),       // #bb9af7
            color_time: "\x1b[38;2;140;165;240m".to_string(),       // #8ca5f0
            color_memory: "\x1b[38;2;158;206;106m".to_string(),     // #9ece6a
            color_separator: "\x1b[38;2;86;95;137m".to_string(),    // #565f89
            color_bg: "\x1b[48;2;26;27;38m".to_string(),            // #1a1b26
            color_tooltip_key: "\x1b[38;2;42;195;222m".to_string(),     // #2ac3de cyan
            color_tooltip_arrow: "\x1b[38;2;86;95;137m".to_string(),    // #565f89 dim
            color_tooltip_action: "\x1b[38;2;187;154;247m".to_string(), // #bb9af7 purple
            color_tooltip_mode: "\x1b[38;2;122;162;247m".to_string(),   // #7aa2f7 blue
            separator: "│".to_string(),
            timezone_offset: 0,
        }
    }
}
