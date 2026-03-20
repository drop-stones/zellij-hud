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
/// Config key for which client_id spawned this HUD/Tooltip instance.
pub(crate) const CONFIG_SPAWNED_FOR_CLIENT: &str = "spawned_for_client";

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
    /// 1-based tab index from spawn config; active clone moves here immediately on permission.
    pub(crate) initial_tab: usize,
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
    /// This instance's client ID (from get_plugin_ids).
    pub(crate) own_client_id: u16,
    /// The client ID that spawned this HUD/Tooltip instance.
    /// Set from plugin config at load time. For Daemon: same as own_client_id.
    /// Only the clone whose own_client_id == spawned_for_client follows tab changes
    /// and resizes, preventing multiple clones from fighting.
    pub(crate) spawned_for_client: u16,
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
            own_client_id: 0,
            spawned_for_client: 0,
            active_tab_idx: 0,
            initial_tab: 0,
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

    /// Broadcast this Daemon instance's current mode to all peers via pipe.
    fn broadcast_mode_sync(&self) {
        pipe_message_to_plugin(
            MessageToPlugin::new("mode_sync")
                .with_payload(format!("{}:{:?}", self.own_client_id, self.mode)),
        );
    }

    /// Handle a "mode_sync:{client_id}:{mode}" pipe message (HUD/Tooltip only).
    /// Daemons ignore this; HUD/Tooltip update their display mode from their spawner's Daemon.
    fn handle_mode_sync_pipe(&mut self, payload: &str) -> bool {
        if self.role == Role::Daemon {
            return false;
        }

        let (id_str, mode_str) = match payload.split_once(':') {
            Some(pair) => pair,
            None => return false,
        };
        let client_id: u16 = match id_str.parse() {
            Ok(id) => id,
            Err(_) => return false,
        };
        // Only react to the Daemon that spawned us.
        if client_id != self.spawned_for_client {
            return false;
        }
        let mode = match mode_from_str(mode_str) {
            Some(m) => m,
            None => return false,
        };

        let base = self.resolve_base_mode();
        if self.mode != mode {
            self.mode = mode;
            if self.role == Role::Tooltip && !is_tooltip_hidden_mode(mode, base) {
                // Only the active clone resizes (uses correct display dimensions).
                if self.own_client_id == self.spawned_for_client {
                    self.resize_tooltip_for_mode();
                    self.update_tooltip_title();
                }
            }
            return true;
        }
        false
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

/// Parse an InputMode from its Debug string representation (e.g. "Normal", "Pane").
fn mode_from_str(s: &str) -> Option<InputMode> {
    match s {
        "Locked" => Some(InputMode::Locked),
        "Normal" => Some(InputMode::Normal),
        "Pane" => Some(InputMode::Pane),
        "Tab" => Some(InputMode::Tab),
        "Resize" => Some(InputMode::Resize),
        "Move" => Some(InputMode::Move),
        "Scroll" => Some(InputMode::Scroll),
        "Search" => Some(InputMode::Search),
        "EnterSearch" => Some(InputMode::EnterSearch),
        "RenameTab" => Some(InputMode::RenameTab),
        "RenamePane" => Some(InputMode::RenamePane),
        "Session" => Some(InputMode::Session),
        "Prompt" => Some(InputMode::Prompt),
        "Tmux" => Some(InputMode::Tmux),
        _ => None,
    }
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
                self.own_client_id = ids.client_id;
                self.cwd = ids.initial_cwd;

                // Determine which client's Daemon spawned us for tab-following.
                // Falls back to own_client_id if missing (single-client scenario).
                self.spawned_for_client = configuration
                    .get(CONFIG_SPAWNED_FOR_CLIENT)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(self.own_client_id);

                // Tab the Daemon was on when it spawned us; active clone moves here immediately.
                self.initial_tab = configuration
                    .get("initial_tab")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1);

                set_selectable(false);
                rename_plugin_pane(ids.plugin_id, "");

                // Attempt to move to initial_tab immediately in load(), before permissions
                // and before the first render cycle. This minimises the window during which
                // the pane is visible on the wrong tab for other clients.
                // Only the active clone (own == spawned) should move the pane.
                if self.own_client_id == self.spawned_for_client {
                    if let Some(plugin_id) = self.own_plugin_id {
                        break_panes_to_tab_with_index(
                            &[PaneId::Plugin(plugin_id)],
                            self.initial_tab.saturating_sub(1),
                            false,
                        );
                        self.active_tab_idx = self.initial_tab;
                    }
                }

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
                self.own_client_id = get_plugin_ids().client_id;
                self.spawned_for_client = self.own_client_id;
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
                // Daemon no longer needs Timer (debounce removed; close is immediate).
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
                        Role::Hud | Role::Tooltip => {
                            // Active clone: move pane to correct tab (backup for load() attempt
                            // in case break_panes_to_tab_with_index requires permissions).
                            if self.own_client_id == self.spawned_for_client {
                                if let Some(plugin_id) = self.own_plugin_id {
                                    let tab_0based = self.initial_tab.saturating_sub(1);
                                    break_panes_to_tab_with_index(
                                        &[PaneId::Plugin(plugin_id)],
                                        tab_0based,
                                        false,
                                    );
                                    self.active_tab_idx = self.initial_tab;
                                }
                            }
                        }
                        Role::Daemon => {
                            hide_self();
                        }
                    }
                    match self.role {
                        Role::Hud => {
                            // Detect local timezone for clock display.
                            let mut tz_ctx = BTreeMap::new();
                            tz_ctx.insert(CMD_CONTEXT_TZ.to_string(), "1".to_string());
                            run_command(&["date", "+%z"], tz_ctx);
                            // Initial memory usage.
                            let mut mem_ctx = BTreeMap::new();
                            mem_ctx.insert(CMD_CONTEXT_MEM.to_string(), "1".to_string());
                            run_command(&["free", "-b"], mem_ctx);
                            // Ask all Daemons for the current mode (active and non-active clones
                            // both need this so they render the correct mode content).
                            pipe_message_to_plugin(MessageToPlugin::new("request_mode_sync"));
                        }
                        Role::Tooltip => {
                            pipe_message_to_plugin(MessageToPlugin::new("request_mode_sync"));
                        }
                        Role::Daemon => {}
                    }
                }
                true
            }
            Event::RunCommandResult(_exit_code, ref stdout, _stderr, ref context) => {
                if context.contains_key(CMD_CONTEXT_TZ) {
                    if let Some(offset) = commands::parse_date_tz(stdout) {
                        // Store timezone directly on hud_config (not plugin_config) so it
                        // doesn't affect get_or_load_plugins config-equality matching.
                        self.hud_config.timezone_offset = offset;
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

                self.session_name = mode_info.session_name.clone().unwrap_or_default();
                self.mode_info = Some(mode_info);

                let base = self.resolve_base_mode();

                match self.role {
                    Role::Hud => {
                        // self.mode is driven exclusively by mode_sync pipe so that
                        // all clients see the same globally active mode.
                    }
                    Role::Tooltip => {
                        // Same: mode driven by mode_sync; don't resize here.
                    }
                    Role::Daemon => {
                        self.mode = new_mode;
                        if self.has_permission {
                            // Broadcast own mode so HUD/Tooltip can display it correctly.
                            self.broadcast_mode_sync();

                            // Spawn/close based exclusively on this client's own mode.
                            if new_mode != base {
                                if self.enable_status_bar && !self.hud_is_open {
                                    self.spawn_hud();
                                }
                                if is_tooltip_hidden_mode(new_mode, base) {
                                    if self.tooltip_is_open {
                                        self.close_tooltip_via_pipe();
                                        self.tooltip_is_open = false;
                                    }
                                } else if self.enable_tooltip && !self.tooltip_is_open {
                                    self.spawn_tooltip(new_mode);
                                }
                            } else {
                                // Mode returned to base: close immediately (no debounce).
                                if self.hud_is_open {
                                    self.close_hud_via_pipe();
                                    self.hud_is_open = false;
                                }
                                if self.tooltip_is_open {
                                    self.close_tooltip_via_pipe();
                                    self.tooltip_is_open = false;
                                }
                            }
                        }
                    }
                }

                let _ = base;
                true
            }
            Event::TabUpdate(tabs) => {
                self.tabs = tabs;

                if self.role == Role::Hud || self.role == Role::Tooltip {
                    // Only the clone spawned for this client (own_client_id == spawned_for_client)
                    // moves the pane. This is known from load() time — no race condition.
                    let is_active_clone = self.own_client_id == self.spawned_for_client;

                    if let Some(active_tab_index) =
                        self.tabs.iter().position(|t| t.active)
                    {
                        let new_idx = active_tab_index + 1;
                        if is_active_clone && self.active_tab_idx != new_idx {
                            if let Some(id) = self.own_plugin_id {
                                break_panes_to_tab_with_index(
                                    &[PaneId::Plugin(id)],
                                    new_idx.saturating_sub(1),
                                    false,
                                );
                            }
                        }
                        self.active_tab_idx = new_idx;
                    }

                    if self.role == Role::Tooltip && is_active_clone {
                        let base = self.resolve_base_mode();
                        if !is_tooltip_hidden_mode(self.mode, base) {
                            self.resize_tooltip_for_mode();
                        }
                    }
                }
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

    fn pipe(&mut self, message: PipeMessage) -> bool {
        let payload = message.payload.as_deref().unwrap_or("");
        match message.name.as_str() {
            "mode_sync" => self.handle_mode_sync_pipe(payload),
            "request_mode_sync" => {
                // HUD/Tooltip asks for current mode on load.
                // Each Daemon responds; HUD/Tooltip will use only their spawner's reply.
                if self.role == Role::Daemon && self.has_permission {
                    self.broadcast_mode_sync();
                }
                false
            }
            "close_hud" => match self.role {
                Role::Hud => {
                    let client_id: u16 = payload.parse().unwrap_or(0);
                    if client_id == self.spawned_for_client {
                        close_self();
                    }
                    false
                }
                Role::Daemon => {
                    let client_id: u16 = payload.parse().unwrap_or(0);
                    if client_id == self.own_client_id {
                        self.hud_is_open = false;
                    }
                    false
                }
                _ => false,
            },
            "close_tooltip" => match self.role {
                Role::Tooltip => {
                    let client_id: u16 = payload.parse().unwrap_or(0);
                    if client_id == self.spawned_for_client {
                        close_self();
                    }
                    false
                }
                Role::Daemon => {
                    let client_id: u16 = payload.parse().unwrap_or(0);
                    if client_id == self.own_client_id {
                        self.tooltip_is_open = false;
                    }
                    false
                }
                _ => false,
            },
            _ => false,
        }
    }

    fn render(&mut self, rows: usize, cols: usize) {
        match self.role {
            Role::Hud => {
                let left = format!(
                    " {}",
                    self.render_format(&self.hud_config.format_left.clone())
                );
                let right = format!(
                    "{} ",
                    self.render_format(&self.hud_config.format_right.clone()),
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
