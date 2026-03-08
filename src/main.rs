mod commands;
mod config;
mod datetime;
mod render;
mod spawn;

use zellij_tile::prelude::*;

use std::collections::BTreeMap;
use std::path::PathBuf;

use commands::{CMD_CONTEXT_MEM, CMD_CONTEXT_TZ, MEM_UPDATE_INTERVAL};
use config::HudConfig;

pub(crate) const CONFIG_IS_HUD: &str = "is_hud";

/// On-demand floating status bar for zellij.
///
/// Architecture: two roles in one plugin binary.
///
/// 1. **Daemon** (is_hud = false): Runs hidden in the background,
///    listens to ModeUpdate events. Spawns/closes HUD instances.
///
/// 2. **HUD** (is_hud = true): Spawned as a floating pane by the daemon.
///    Renders the status bar. Closes itself when mode returns to Locked.
pub(crate) struct State {
    pub(crate) is_hud: bool,
    pub(crate) mode: InputMode,
    pub(crate) mode_info: Option<ModeInfo>,
    pub(crate) tabs: Vec<TabInfo>,
    pub(crate) has_permission: bool,
    pub(crate) hud_is_open: bool,
    /// HUD: own plugin ID for self-movement across tabs
    pub(crate) own_plugin_id: Option<u32>,
    /// HUD: 1-based index of the tab the HUD is currently on
    pub(crate) active_tab_idx: usize,
    /// HUD: initial CWD of the plugin
    pub(crate) cwd: PathBuf,
    /// HUD: session name
    pub(crate) session_name: String,
    /// Raw plugin config from load(), forwarded to HUD instances
    pub(crate) plugin_config: BTreeMap<String, String>,
    /// Parsed configuration (HUD only)
    pub(crate) hud_config: HudConfig,
    /// HUD: formatted memory usage string
    pub(crate) memory_text: String,
    /// HUD: timer tick counter for throttling memory updates
    pub(crate) timer_count: u32,
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
            memory_text: String::new(),
            timer_count: 0,
        }
    }
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.is_hud = configuration
            .get(CONFIG_IS_HUD)
            .map_or(false, |v| v == "true");

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
                PermissionType::MessageAndLaunchOtherPlugins,
                PermissionType::RunCommands,
            ]);
            subscribe(&[
                EventType::ModeUpdate,
                EventType::TabUpdate,
                EventType::Timer,
                EventType::PermissionRequestResult,
                EventType::RunCommandResult,
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
                PermissionType::RunCommands,
            ]);
            subscribe(&[
                EventType::ModeUpdate,
                EventType::TabUpdate,
                EventType::PermissionRequestResult,
                EventType::RunCommandResult,
            ]);
        }
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(result) => {
                if result == PermissionStatus::Granted {
                    self.has_permission = true;
                    if self.is_hud {
                        // Fetch memory usage immediately on startup
                        let mut ctx = BTreeMap::new();
                        ctx.insert(CMD_CONTEXT_MEM.to_string(), "1".to_string());
                        run_command(&["free", "-b"], ctx);
                    } else {
                        hide_self();
                        // Detect timezone via `date +%z` (requires `date` on host)
                        let mut ctx = BTreeMap::new();
                        ctx.insert(CMD_CONTEXT_TZ.to_string(), "1".to_string());
                        run_command(&["date", "+%z"], ctx);
                    }
                }
                true
            }
            Event::RunCommandResult(_exit_code, ref stdout, _stderr, ref context) => {
                if context.contains_key(CMD_CONTEXT_TZ) {
                    if let Some(offset) = commands::parse_date_tz(stdout) {
                        self.plugin_config
                            .insert("timezone".to_string(), offset.to_string());
                    }
                } else if context.contains_key(CMD_CONTEXT_MEM) {
                    if let Some((used, total)) = commands::parse_free(stdout) {
                        let pct = (used as f64 / total as f64) * 100.0;
                        self.memory_text = format!("{:.0}%", pct);
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
                    self.session_name = mode_info.session_name.clone().unwrap_or_default();
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
                    self.timer_count += 1;
                    // Update memory usage every MEM_UPDATE_INTERVAL seconds
                    if self.timer_count % MEM_UPDATE_INTERVAL == 1 {
                        let mut ctx = BTreeMap::new();
                        ctx.insert(CMD_CONTEXT_MEM.to_string(), "1".to_string());
                        run_command(&["free", "-b"], ctx);
                    }
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

        let left = format!(
            "{bg} {}{reset}",
            self.render_format(&self.hud_config.format_left.clone())
        );
        let right = format!(
            "{}{} {reset}",
            self.render_format(&self.hud_config.format_right.clone()),
            ""
        );

        let left_visible = Self::visible_len(&left);
        let right_visible = Self::visible_len(&right);
        let gap = cols.saturating_sub(left_visible + right_visible);

        print!("{}{}{}", left, " ".repeat(gap), right);
    }
}
