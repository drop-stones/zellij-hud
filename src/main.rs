use zellij_tile::prelude::*;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const CONFIG_IS_HUD: &str = "is_hud";

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
        }
    }
}

register_plugin!(State);

impl State {
    fn spawn_hud(&mut self) {
        if self.hud_is_open {
            return;
        }
        let mut config = BTreeMap::new();
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
            let mins = (total_secs / 60) % 60;
            let hours = (total_secs / 3600) % 24;
            let hours_jst = (hours + 9) % 24;
            format!("{:02}:{:02}", hours_jst, mins)
        } else {
            "--:--".to_string()
        }
    }

    fn format_date(&self) -> String {
        if let Ok(dur) = SystemTime::now().duration_since(UNIX_EPOCH) {
            let jst_secs = dur.as_secs() + 9 * 3600;
            let jst_days = (jst_secs / 86400) as i64;

            let (_year, month, day) = Self::days_to_ymd(jst_days);
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
}

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.is_hud = configuration.get(CONFIG_IS_HUD).map_or(false, |v| v == "true");

        if self.is_hud {
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

        // ANSI color helpers using theme RGB values
        // dim: #565f89 (comment color from tokyonight)
        let dim = "\x1b[38;2;86;95;137m";
        // Colors from tokyonight-night theme
        let cyan = "\x1b[38;2;42;195;222m";
        let blue = "\x1b[38;2;140;165;240m";
        let magenta = "\x1b[38;2;187;154;247m";
        let base = "\x1b[38;2;192;202;245m";
        let bg = "\x1b[48;2;26;27;38m";
        let reset = "\x1b[0m";

        let sep = format!("{}│{}", dim, reset);

        // === Left side ===
        let mut left = format!(
            "{bg} {cyan}󰆍 {}{reset}{bg} {sep}{bg} {blue}{} {}{reset}{bg} {sep}{bg} ",
            self.session_name,
            self.mode_icon(),
            format!("{:?}", self.mode).to_uppercase(),
        );

        // Tabs (no separators, just spacing)
        for tab in &self.tabs {
            if tab.active {
                left.push_str(&format!("{base} {} {reset}{bg}", tab.name));
            } else {
                left.push_str(&format!("{dim} {} {reset}{bg}", tab.name));
            }
        }

        // === Right side ===
        let right = format!(
            "{cyan}󰉖 {}{reset}{bg} {sep}{bg} {magenta}󰃭 {}{reset}{bg} {sep}{bg} {blue}󰥔 {} {reset}",
            self.format_cwd(),
            self.format_date(),
            self.format_time(),
        );

        // Calculate visible widths (strip ANSI for counting)
        let left_visible = Self::visible_len(&left);
        let right_visible = Self::visible_len(&right);
        let gap = cols.saturating_sub(left_visible + right_visible);

        print!("{}{}{}", left, " ".repeat(gap), right);
    }
}
