mod action_types;
mod commands;
mod config;
mod datetime;
mod keybinds;
pub(crate) mod render;
mod spawn;
mod tooltip;

use zellij_tile::prelude::*;

use std::collections::BTreeMap;
use std::path::PathBuf;

use commands::{CMD_CONTEXT_MEM, CMD_CONTEXT_TZ, MEM_UPDATE_INTERVAL};
use config::{BaseMode, HudConfig};
use render::visible_len;

pub(crate) const CONFIG_IS_HUD: &str = "is_hud";
pub(crate) const CONFIG_IS_TOOLTIP: &str = "is_tooltip";

/// Plugin role within the zellij-hud system.
#[derive(Default, PartialEq)]
pub(crate) enum Role {
    /// Background daemon that spawns HUD and tooltip panes.
    #[default]
    Daemon,
    /// Floating status bar at the bottom.
    Hud,
    /// Floating which-key tooltip at the bottom-right.
    Tooltip,
}

/// On-demand floating status bar and keybinding tooltip for zellij.
///
/// Architecture: three roles in one plugin binary.
///
/// 1. **Daemon**: Runs hidden in the background,
///    listens to ModeUpdate events. Spawns/closes HUD and Tooltip instances.
///
/// 2. **HUD**: Spawned as a floating pane by the daemon.
///    Renders the status bar. Closes itself when mode returns to Locked.
///
/// 3. **Tooltip**: Spawned as a floating pane by the daemon.
///    Renders which-key style keybinding hints. Dynamically resizes itself
///    on mode changes via `change_floating_panes_coordinates`.
pub(crate) struct State {
    pub(crate) role: Role,
    pub(crate) mode: InputMode,
    pub(crate) mode_info: Option<ModeInfo>,
    pub(crate) tabs: Vec<TabInfo>,
    pub(crate) has_permission: bool,
    pub(crate) hud_is_open: bool,
    pub(crate) tooltip_is_open: bool,
    /// Own plugin ID for self-movement across tabs
    pub(crate) own_plugin_id: Option<u32>,
    /// 1-based index of the tab the pane is currently on
    pub(crate) active_tab_idx: usize,
    /// Initial CWD of the plugin
    pub(crate) cwd: PathBuf,
    /// Session name
    pub(crate) session_name: String,
    /// Raw plugin config from load(), forwarded to spawned instances
    pub(crate) plugin_config: BTreeMap<String, String>,
    /// Parsed configuration
    pub(crate) hud_config: HudConfig,
    /// Whether the status bar is enabled
    pub(crate) enable_status_bar: bool,
    /// Whether the tooltip is enabled
    pub(crate) enable_tooltip: bool,
    /// Base mode config setting (override for ModeInfo::base_mode)
    pub(crate) base_mode_config: BaseMode,
    /// Formatted memory usage string
    pub(crate) memory_text: String,
    /// Timer tick counter for throttling memory updates
    pub(crate) timer_count: u32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            role: Role::Daemon,
            mode: InputMode::Locked,
            mode_info: None,
            tabs: Vec::new(),
            has_permission: false,
            hud_is_open: false,
            tooltip_is_open: false,
            own_plugin_id: None,
            active_tab_idx: 0,
            cwd: PathBuf::new(),
            session_name: String::new(),
            plugin_config: BTreeMap::new(),
            hud_config: HudConfig::default(),
            enable_status_bar: true,
            enable_tooltip: true,
            base_mode_config: BaseMode::Auto,
            memory_text: String::new(),
            timer_count: 0,
        }
    }
}

impl State {
    /// Resolve the base mode from ModeInfo or config override.
    fn resolve_base_mode(&self) -> InputMode {
        // Explicit config override takes priority
        let config_base = match self.role {
            Role::Daemon => self.base_mode_config,
            Role::Hud | Role::Tooltip => self.hud_config.base_mode,
        };
        match config_base {
            BaseMode::Locked => InputMode::Locked,
            BaseMode::Normal => InputMode::Normal,
            BaseMode::Auto => self
                .mode_info
                .as_ref()
                .and_then(|mi| mi.base_mode)
                .unwrap_or(InputMode::Normal),
        }
    }
}

/// Modes where the tooltip should not be shown (base mode + text input modes).
fn is_tooltip_hidden_mode(mode: InputMode, base_mode: InputMode) -> bool {
    mode == base_mode
        || matches!(
            mode,
            InputMode::RenamePane | InputMode::RenameTab | InputMode::EnterSearch
        )
}

register_plugin!(State);

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        if configuration
            .get(CONFIG_IS_HUD)
            .map_or(false, |v| v == "true")
        {
            self.role = Role::Hud;
        } else if configuration
            .get(CONFIG_IS_TOOLTIP)
            .map_or(false, |v| v == "true")
        {
            self.role = Role::Tooltip;
        }

        match self.role {
            Role::Hud | Role::Tooltip => {
                self.hud_config = HudConfig::from_config(&configuration);

                let ids = get_plugin_ids();
                self.own_plugin_id = Some(ids.plugin_id);
                self.cwd = ids.initial_cwd;

                set_selectable(false);
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

                if self.role == Role::Hud {
                    set_timeout(1.0);
                }
            }
            Role::Daemon => {
                self.enable_status_bar =
                    configuration.get("enable_status_bar").map_or(true, |v| v != "false");
                self.enable_tooltip =
                    configuration.get("enable_tooltip").map_or(true, |v| v != "false");
                self.base_mode_config = match configuration.get("base_mode").map(|s| s.as_str()) {
                    Some("locked") => BaseMode::Locked,
                    Some("normal") => BaseMode::Normal,
                    _ => BaseMode::Auto,
                };
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
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(result) => {
                if result == PermissionStatus::Granted {
                    self.has_permission = true;
                    match self.role {
                        Role::Hud => {
                            let mut ctx = BTreeMap::new();
                            ctx.insert(CMD_CONTEXT_MEM.to_string(), "1".to_string());
                            run_command(&["free", "-b"], ctx);
                        }
                        Role::Tooltip => {}
                        Role::Daemon => {
                            hide_self();
                            let mut ctx = BTreeMap::new();
                            ctx.insert(CMD_CONTEXT_TZ.to_string(), "1".to_string());
                            run_command(&["date", "+%z"], ctx);
                        }
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

                // Store mode_info first so spawn functions can use it
                self.session_name =
                    mode_info.session_name.clone().unwrap_or_default();
                self.mode = new_mode;
                self.mode_info = Some(mode_info);

                // Resolve base mode on first ModeUpdate (needs keybindings)
                let base = self.resolve_base_mode();

                match self.role {
                    Role::Hud => {
                        if new_mode == base {
                            close_self();
                            return false;
                        }
                    }
                    Role::Tooltip => {
                        if is_tooltip_hidden_mode(new_mode, base) {
                            close_self();
                            return false;
                        }
                        self.resize_tooltip_for_mode();
                        self.update_tooltip_title();
                    }
                    Role::Daemon => {
                        if self.has_permission {
                            if new_mode != base {
                                if self.enable_status_bar && !self.hud_is_open {
                                    self.spawn_hud();
                                }
                                if self.enable_tooltip
                                    && !is_tooltip_hidden_mode(new_mode, base)
                                    && !self.tooltip_is_open
                                {
                                    self.spawn_tooltip();
                                }
                                if is_tooltip_hidden_mode(new_mode, base) {
                                    self.tooltip_is_open = false;
                                }
                            } else {
                                self.hud_is_open = false;
                                self.tooltip_is_open = false;
                            }
                        }
                    }
                }

                true
            }
            Event::TabUpdate(tabs) => {
                if self.role == Role::Hud || self.role == Role::Tooltip {
                    if let Some(active_tab_index) =
                        tabs.iter().position(|t| t.active)
                    {
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
                if self.role == Role::Hud {
                    set_timeout(1.0);
                    self.timer_count += 1;
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

    fn render(&mut self, rows: usize, cols: usize) {
        match self.role {
            Role::Hud => {
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

                let left_visible = visible_len(&left);
                let right_visible = visible_len(&right);
                let gap = cols.saturating_sub(left_visible + right_visible);

                print!("{}{}{}", left, " ".repeat(gap), right);
            }
            Role::Tooltip => {
                self.render_tooltip(rows, cols);
            }
            Role::Daemon => {}
        }
    }
}
