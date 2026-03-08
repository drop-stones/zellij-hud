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
            .new_plugin_instance_should_have_pane_title("hud".to_string());

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
            // UTC time components
            let secs = total_secs % 60;
            let mins = (total_secs / 60) % 60;
            let hours = (total_secs / 3600) % 24;
            // Adjust for JST (UTC+9)
            let hours_jst = (hours + 9) % 24;
            format!("{:02}:{:02}:{:02}", hours_jst, mins, secs)
        } else {
            "--:--:--".to_string()
        }
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

        // === Left side ===
        // Session name
        let session_seg = format!(" 󰆍 {} ", self.session_name);
        let sep = " │ ";
        // Mode
        let mode_name = format!("{:?}", self.mode).to_uppercase();
        let mode_seg = format!("{} {} ", self.mode_icon(), mode_name);
        // Tabs
        let tab_parts: Vec<(String, bool)> = self.tabs.iter().map(|t| {
            let label = format!(" {} ", t.name);
            (label, t.active)
        }).collect();

        let mut left = session_seg.clone();
        left.push_str(sep);
        left.push_str(&mode_seg);
        left.push_str(sep);
        left.push_str("󰓩");
        for (i, (name, _)) in tab_parts.iter().enumerate() {
            if i > 0 {
                left.push_str("│");
            }
            left.push_str(name);
        }

        // === Right side ===
        let cwd_seg = format!(" 󰉖 {} ", self.format_cwd());
        let time_seg = format!(" 󰥔 {} ", self.format_time());
        let right = format!("{}{}{}", sep, cwd_seg, time_seg);

        // === Compose full line ===
        let left_chars = left.chars().count();
        let right_chars = right.chars().count();
        let gap = cols.saturating_sub(left_chars + right_chars);
        let line = format!("{}{}{}", left, " ".repeat(gap), right);

        // === Style ===
        let mut text = Text::new(&line).opaque();

        // Session name: emphasis_1 (index 3)
        let session_chars = session_seg.chars().count();
        text = text.color_range(3, 0..session_chars);

        // Mode: emphasis_2 (index 4)
        let mut pos = session_chars + sep.chars().count();
        let mode_seg_chars = mode_seg.chars().count();
        text = text.color_range(4, pos..pos + mode_seg_chars);
        pos += mode_seg_chars;

        // Tab icon + separators: base color
        pos += sep.chars().count();
        let tab_icon_chars = "󰓩".chars().count();
        text = text.color_range(0, pos..pos + tab_icon_chars);
        pos += tab_icon_chars;

        // Tab names: active = emphasis_0, inactive = no color
        for (i, (name, active)) in tab_parts.iter().enumerate() {
            if i > 0 {
                pos += "│".chars().count();
            }
            let name_chars = name.chars().count();
            if *active {
                text = text.color_range(0, pos..pos + name_chars);
            }
            pos += name_chars;
        }

        // Right side: CWD and time
        let right_start = left_chars + gap;
        let right_sep_chars = sep.chars().count();
        let cwd_start = right_start + right_sep_chars;
        let cwd_chars = cwd_seg.chars().count();
        text = text.color_range(2, cwd_start..cwd_start + cwd_chars);

        let time_start = cwd_start + cwd_chars;
        let time_chars = time_seg.chars().count();
        text = text.color_range(3, time_start..time_start + time_chars);

        print_text(text);
    }
}
