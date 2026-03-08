use zellij_tile::prelude::*;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const CONFIG_IS_HUD: &str = "is_hud";

struct HudConfig {
    format_left: String,
    format_right: String,
    color_session: String,
    color_mode: String,
    color_tab_active: String,
    color_tab_inactive: String,
    color_cwd: String,
    color_date: String,
    color_time: String,
    color_separator: String,
    color_bg: String,
    separator: String,
    timezone_offset: i64,
}

impl HudConfig {
    fn from_config(config: &BTreeMap<String, String>) -> Self {
        let mut hud = Self::default();

        if let Some(v) = config.get("format_left") {
            hud.format_left = v.clone();
        }
        if let Some(v) = config.get("format_right") {
            hud.format_right = v.clone();
        }
        if let Some(v) = config.get("color_session") {
            if let Some(c) = Self::hex_to_fg(v) { hud.color_session = c; }
        }
        if let Some(v) = config.get("color_mode") {
            if let Some(c) = Self::hex_to_fg(v) { hud.color_mode = c; }
        }
        if let Some(v) = config.get("color_tab_active") {
            if let Some(c) = Self::hex_to_fg(v) { hud.color_tab_active = c; }
        }
        if let Some(v) = config.get("color_tab_inactive") {
            if let Some(c) = Self::hex_to_fg(v) { hud.color_tab_inactive = c; }
        }
        if let Some(v) = config.get("color_cwd") {
            if let Some(c) = Self::hex_to_fg(v) { hud.color_cwd = c; }
        }
        if let Some(v) = config.get("color_date") {
            if let Some(c) = Self::hex_to_fg(v) { hud.color_date = c; }
        }
        if let Some(v) = config.get("color_time") {
            if let Some(c) = Self::hex_to_fg(v) { hud.color_time = c; }
        }
        if let Some(v) = config.get("color_separator") {
            if let Some(c) = Self::hex_to_fg(v) { hud.color_separator = c; }
        }
        if let Some(v) = config.get("color_bg") {
            if let Some(c) = Self::hex_to_bg(v) { hud.color_bg = c; }
        }
        if let Some(v) = config.get("separator") {
            hud.separator = v.clone();
        }
        if let Some(v) = config.get("timezone") {
            if let Ok(n) = v.parse::<i64>() { hud.timezone_offset = n; }
        }

        hud
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
            format_right: "{cwd} | {date} | {time}".to_string(),
            color_session: "\x1b[38;2;42;195;222m".to_string(),   // #2ac3de
            color_mode: "\x1b[38;2;140;165;240m".to_string(),     // #8ca5f0
            color_tab_active: "\x1b[38;2;192;202;245m".to_string(), // #c0caf5
            color_tab_inactive: "\x1b[38;2;86;95;137m".to_string(), // #565f89
            color_cwd: "\x1b[38;2;42;195;222m".to_string(),       // #2ac3de
            color_date: "\x1b[38;2;187;154;247m".to_string(),     // #bb9af7
            color_time: "\x1b[38;2;140;165;240m".to_string(),     // #8ca5f0
            color_separator: "\x1b[38;2;86;95;137m".to_string(),  // #565f89
            color_bg: "\x1b[48;2;26;27;38m".to_string(),          // #1a1b26
            separator: "│".to_string(),
            timezone_offset: 0,
        }
    }
}

/// On-demand floating status bar for zellij.
///
/// Architecture: two roles in one plugin binary.
///
/// 1. **Daemon** (is_hud = false): Runs hidden in the background,
///    listens to ModeUpdate events. Spawns/closes HUD instances.
///
/// 2. **HUD** (is_hud = true): Spawned as a floating pane by the daemon.
///    Renders the status bar. Closes itself when mode returns to Locked.
struct State {
    is_hud: bool,
    mode: InputMode,
    mode_info: Option<ModeInfo>,
    tabs: Vec<TabInfo>,
    has_permission: bool,
    hud_is_open: bool,
    /// HUD: own plugin ID for self-movement across tabs
    own_plugin_id: Option<u32>,
    /// HUD: 1-based index of the tab the HUD is currently on
    active_tab_idx: usize,
    /// HUD: initial CWD of the plugin
    cwd: PathBuf,
    /// HUD: session name
    session_name: String,
    /// Raw plugin config from load(), forwarded to HUD instances
    plugin_config: BTreeMap<String, String>,
    /// Parsed configuration (HUD only)
    hud_config: HudConfig,
}

impl Default for State {
    fn default() -> Self {
        Self {
            is_hud: false,
            mode: InputMode::Locked,
            mode_info: None,
            tabs: Vec::new(),
            has_permission: false,
            hud_is_open: false,
            own_plugin_id: None,
            active_tab_idx: 0,
            cwd: PathBuf::new(),
            session_name: String::new(),
            plugin_config: BTreeMap::new(),
            hud_config: HudConfig::default(),
        }
    }
}

register_plugin!(State);

impl State {
    fn spawn_hud(&mut self) {
        if self.hud_is_open {
            return;
        }
        let mut config = self.plugin_config.clone();
        config.insert(CONFIG_IS_HUD.to_string(), "true".to_string());

        let msg = MessageToPlugin::new("spawn_hud")
            .with_plugin_url("zellij:OWN_URL")
            .with_plugin_config(config)
            .with_floating_pane_coordinates(self.hud_coordinates())
            .new_plugin_instance_should_have_pane_title(String::new());

        pipe_message_to_plugin(msg);
        self.hud_is_open = true;
    }

    fn hud_coordinates(&self) -> FloatingPaneCoordinates {
        let (rows, cols) = self.tabs.iter()
            .find(|t| t.active)
            .map(|t| (t.display_area_rows, t.display_area_columns))
            .unwrap_or((24, 80));

        let height = 3;
        let y = rows.saturating_sub(height);

        FloatingPaneCoordinates::new(
            Some("0".to_string()),
            Some(format!("{}", y)),
            Some(format!("{}", cols)),
            Some(format!("{}", height)),
            Some(true),
        ).unwrap_or_default()
    }

    fn format_time(&self) -> String {
        if let Ok(dur) = SystemTime::now().duration_since(UNIX_EPOCH) {
            let total_secs = dur.as_secs();
            let offset = self.hud_config.timezone_offset;
            let adjusted = (total_secs as i64 + offset * 3600).rem_euclid(86400);
            let hours = adjusted / 3600;
            let mins = (adjusted % 3600) / 60;
            format!("{:02}:{:02}", hours, mins)
        } else {
            "--:--".to_string()
        }
    }

    fn format_date(&self) -> String {
        if let Ok(dur) = SystemTime::now().duration_since(UNIX_EPOCH) {
            let offset = self.hud_config.timezone_offset;
            let adjusted_secs = dur.as_secs() as i64 + offset * 3600;
            let days = adjusted_secs.div_euclid(86400);

            let (_year, month, day) = Self::days_to_ymd(days);
            let month_name = match month {
                1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr",
                5 => "May", 6 => "Jun", 7 => "Jul", 8 => "Aug",
                9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
                _ => "???",
            };
            format!("{} {:02}", month_name, day)
        } else {
            "--- --".to_string()
        }
    }

    fn days_to_ymd(days_since_epoch: i64) -> (i64, u32, u32) {
        let z = days_since_epoch + 719468;
        let era = z.div_euclid(146097);
        let doe = z.rem_euclid(146097) as u64;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
        let y = yoe as i64 + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let y = if m <= 2 { y + 1 } else { y };
        (y, m, d)
    }

    fn format_cwd(&self) -> String {
        if let Some(name) = self.cwd.file_name() {
            name.to_string_lossy().to_string()
        } else {
            self.cwd.to_string_lossy().to_string()
        }
    }

    fn mode_icon(&self) -> &str {
        match self.mode {
            InputMode::Normal => "󰰓",
            InputMode::Locked => "󰌾",
            InputMode::Pane => "󰘖",
            InputMode::Tab => "󰓩",
            InputMode::Resize => "󰩨",
            InputMode::Move => "󰆾",
            InputMode::Scroll => "󰠶",
            InputMode::Session => "󱂬",
            InputMode::Search => "󰍉",
            InputMode::RenameTab => "󰏫",
            InputMode::RenamePane => "󰏫",
            InputMode::EnterSearch => "󰍉",
            InputMode::Tmux => "󰰣",
            InputMode::Prompt => "󰘥",
        }
    }

    fn visible_len(s: &str) -> usize {
        let mut len = 0;
        let mut in_escape = false;
        for ch in s.chars() {
            if ch == '\x1b' {
                in_escape = true;
            } else if in_escape {
                if ch == 'm' {
                    in_escape = false;
                }
            } else {
                len += 1;
            }
        }
        len
    }

    fn render_segment(&self, placeholder: &str) -> String {
        let c = &self.hud_config;
        let bg = &c.color_bg;
        let reset = "\x1b[0m";

        match placeholder {
            "{session}" => {
                format!("{}󰆍 {}{reset}{bg}", c.color_session, self.session_name)
            }
            "{mode}" => {
                format!(
                    "{}{} {}{reset}{bg}",
                    c.color_mode,
                    self.mode_icon(),
                    format!("{:?}", self.mode).to_uppercase(),
                )
            }
            "{tabs}" => {
                let mut out = String::new();
                for tab in &self.tabs {
                    if tab.active {
                        out.push_str(&format!("{} {} {reset}{bg}", c.color_tab_active, tab.name));
                    } else {
                        out.push_str(&format!("{} {} {reset}{bg}", c.color_tab_inactive, tab.name));
                    }
                }
                out
            }
            "{cwd}" => {
                format!("{}󰉖 {}{reset}{bg}", c.color_cwd, self.format_cwd())
            }
            "{date}" => {
                format!("{}󰃭 {}{reset}{bg}", c.color_date, self.format_date())
            }
            "{time}" => {
                format!("{}󰥔 {}{reset}{bg}", c.color_time, self.format_time())
            }
            _ => String::new(),
        }
    }

    fn render_format(&self, format_str: &str) -> String {
        let c = &self.hud_config;
        let bg = &c.color_bg;
        let reset = "\x1b[0m";
        let sep = format!("{}{}{reset}", c.color_separator, c.separator);

        let parts: Vec<&str> = format_str.split(" | ").collect();
        let mut out = String::new();

        for (i, part) in parts.iter().enumerate() {
            let trimmed = part.trim();
            out.push_str(&self.render_segment(trimmed));
            if i < parts.len() - 1 {
                out.push_str(&format!(" {sep}{bg} "));
            }
        }

        out
    }
}

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.is_hud = configuration.get(CONFIG_IS_HUD).map_or(false, |v| v == "true");

        if self.is_hud {
            self.hud_config = HudConfig::from_config(&configuration);

            let ids = get_plugin_ids();
            self.own_plugin_id = Some(ids.plugin_id);
            self.cwd = ids.initial_cwd;

            // Make HUD non-selectable (prevents mouse hover focus)
            set_selectable(false);
            // Clear the frame title
            rename_plugin_pane(ids.plugin_id, "");

            request_permission(&[
                PermissionType::ReadApplicationState,
                PermissionType::ChangeApplicationState,
            ]);
            subscribe(&[
                EventType::ModeUpdate,
                EventType::TabUpdate,
                EventType::Timer,
                EventType::PermissionRequestResult,
            ]);

            // Start clock timer
            set_timeout(1.0);
        } else {
            // Daemon: store config to forward to HUD instances
            self.plugin_config = configuration;

            request_permission(&[
                PermissionType::ReadApplicationState,
                PermissionType::ChangeApplicationState,
                PermissionType::MessageAndLaunchOtherPlugins,
            ]);
            subscribe(&[
                EventType::ModeUpdate,
                EventType::TabUpdate,
                EventType::PermissionRequestResult,
            ]);
        }
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(result) => {
                if result == PermissionStatus::Granted {
                    self.has_permission = true;
                    if !self.is_hud {
                        hide_self();
                    }
                }
                true
            }
            Event::ModeUpdate(mode_info) => {
                let new_mode = mode_info.mode;

                if self.is_hud {
                    if new_mode == InputMode::Locked {
                        close_self();
                        return false;
                    }
                    self.session_name = mode_info.session_name
                        .clone()
                        .unwrap_or_default();
                } else if self.has_permission {
                    if new_mode != InputMode::Locked && !self.hud_is_open {
                        self.spawn_hud();
                    } else if new_mode == InputMode::Locked {
                        self.hud_is_open = false;
                    }
                }

                self.mode = new_mode;
                self.mode_info = Some(mode_info);
                true
            }
            Event::TabUpdate(tabs) => {
                if self.is_hud {
                    if let Some(active_tab_index) = tabs.iter().position(|t| t.active) {
                        let new_idx = active_tab_index + 1;
                        if self.active_tab_idx != new_idx {
                            if let Some(id) = self.own_plugin_id {
                                break_panes_to_tab_with_index(
                                    &[PaneId::Plugin(id)],
                                    new_idx.saturating_sub(1),
                                    false,
                                );
                            }
                            self.active_tab_idx = new_idx;
                        }
                    }
                }
                self.tabs = tabs;
                true
            }
            Event::Timer(_) => {
                if self.is_hud {
                    set_timeout(1.0);
                }
                true
            }
            _ => false,
        }
    }

    fn pipe(&mut self, _pipe_message: PipeMessage) -> bool {
        false
    }

    fn render(&mut self, _rows: usize, cols: usize) {
        if !self.is_hud {
            return;
        }

        let bg = &self.hud_config.color_bg;
        let reset = "\x1b[0m";

        let left = format!("{bg} {}{reset}", self.render_format(&self.hud_config.format_left.clone()));
        let right = format!("{}{} {reset}", self.render_format(&self.hud_config.format_right.clone()), "");

        let left_visible = Self::visible_len(&left);
        let right_visible = Self::visible_len(&right);
        let gap = cols.saturating_sub(left_visible + right_visible);

        print!("{}{}{}", left, " ".repeat(gap), right);
    }
}
