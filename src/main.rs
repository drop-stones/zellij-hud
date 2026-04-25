mod action_types;
mod commands;
mod config;
mod keybinds;
pub(crate) mod render;
mod spans;
mod spawn;
mod tooltip;

use zellij_tile::prelude::*;

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use std::borrow::Cow;

use commands::{CMD_CONTEXT_USER, CommandOutput};

/// Single-quote a string for safe use in sh -c commands.
fn shell_escape(s: &str) -> Cow<'_, str> {
    if s.is_empty() {
        return Cow::Borrowed("''");
    }
    if s.bytes().all(|b| matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-' | b'.' | b'/')) {
        Cow::Borrowed(s)
    } else {
        Cow::Owned(format!("'{}'", s.replace('\'', "'\\''")))
    }
}
use config::HudConfig;
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
    /// Output from user-defined command widgets, keyed by widget name.
    pub(crate) command_outputs: HashMap<String, CommandOutput>,
    /// Per-command timer counters for interval tracking.
    pub(crate) command_timers: HashMap<String, u32>,
    /// This instance's client ID (from get_plugin_ids).
    pub(crate) own_client_id: u16,
    /// The client ID that spawned this HUD/Tooltip instance.
    /// Set from plugin config at load time. For Daemon: same as own_client_id.
    /// Only the clone whose own_client_id == spawned_for_client follows tab changes
    /// and resizes, preventing multiple clones from fighting.
    pub(crate) spawned_for_client: u16,
    /// Whether run_deferred_init() has already executed (prevents double-execution).
    pub(crate) init_done: bool,
    /// Last (height, width) used for tooltip pane — avoids redundant coordinate changes.
    pub(crate) last_tooltip_size: (usize, usize),
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
            command_outputs: HashMap::new(),
            command_timers: HashMap::new(),
            init_done: false,
            last_tooltip_size: (0, 0),
        }
    }
}

impl State {
    /// Resolve the base mode from ModeInfo or config override.
    fn resolve_base_mode(&self) -> InputMode {
        self.mode_info
            .as_ref()
            .and_then(|mi| mi.base_mode)
            .unwrap_or(InputMode::Normal)
    }

    /// Update cwd from the currently focused terminal pane.
    fn update_cwd_from_focused_pane(&mut self) {
        if let Ok((_tab_idx, pane_id)) = get_focused_pane_info() {
            if let PaneId::Terminal(_) = pane_id {
                if let Ok(new_cwd) = get_pane_cwd(pane_id) {
                    self.cwd = new_cwd;
                }
            }
        }
    }

    /// Run all user-defined command widgets immediately.
    fn run_all_command_widgets(&self) {
        for (name, widget) in &self.hud_config.command_widgets {
            self.run_command_widget(name, &widget.command);
        }
    }

    /// Tick per-command interval counters. Run commands whose interval has elapsed.
    fn tick_command_widgets(&mut self) {
        let names_and_cmds: Vec<(String, String, u32)> = self
            .hud_config
            .command_widgets
            .iter()
            .filter(|(_, w)| w.interval > 0)
            .map(|(name, w)| (name.clone(), w.command.clone(), w.interval))
            .collect();

        for (name, command, interval) in names_and_cmds {
            let counter = self.command_timers.entry(name.clone()).or_insert(0);
            *counter += 1;
            if *counter >= interval {
                *counter = 0;
                self.run_command_widget(&name, &command);
            }
        }
    }

    /// Execute a user-defined command widget's shell command in the focused pane's cwd.
    fn run_command_widget(&self, name: &str, command: &str) {
        let mut ctx = BTreeMap::new();
        ctx.insert(CMD_CONTEXT_USER.to_string(), name.to_string());
        let cwd = self.cwd.to_string_lossy();
        let full_cmd = if cwd.is_empty() {
            command.to_string()
        } else {
            format!("cd {} && {}", shell_escape(&cwd), command)
        };
        run_command(&["sh", "-c", &full_cmd], ctx);
    }

    /// One-time setup after permission is granted.
    ///
    /// Called from PermissionRequestResult(Granted).  Only uses
    /// fire-and-forget APIs (no `bytes_from_stdin().unwrap()`) so it
    /// won't panic even if the host hasn't fully applied permissions
    /// due to a race condition.  Guarded by `init_done`.
    ///
    /// Tab-following (`break_panes_to_tab_with_index`) and CWD update
    /// (`get_focused_pane_info`) are handled by TabUpdate and Timer
    /// respectively, since their shims panic on permission denial.
    fn run_deferred_init(&mut self) {
        if self.init_done {
            return;
        }
        self.init_done = true;
        match self.role {
            Role::Hud => {
                if let Some(plugin_id) = self.own_plugin_id {
                    rename_plugin_pane(plugin_id, "");
                }
            }
            // Tooltip: title is drawn manually in the border; no pane rename needed.
            Role::Tooltip => {}
            Role::Daemon => {
                hide_self();
            }
        }
        match self.role {
            Role::Hud => {
                pipe_message_to_plugin(MessageToPlugin::new("request_mode_sync"));
            }
            Role::Tooltip => {
                pipe_message_to_plugin(MessageToPlugin::new("request_mode_sync"));
            }
            Role::Daemon => {
                // Start running command widgets now that permission is granted.
                // Results are stored and forwarded to HUD via pipe/config.
                self.run_all_command_widgets();
            }
        }
    }

    /// Spawn HUD/Tooltip if the Daemon is in a non-base mode and all required state
    /// (tabs, mode_info) is available.  Called from PermissionRequestResult and
    /// TabUpdate to recover from the race where ModeUpdate arrived before permission
    /// was granted and the spawn was skipped.
    fn daemon_try_spawn(&mut self) {
        if self.role != Role::Daemon || !self.has_permission {
            return;
        }
        // mode_info being None means no ModeUpdate has arrived yet; self.mode is
        // stale (default Locked).  Wait until mode_info is populated.
        if self.mode_info.is_none() || self.tabs.is_empty() {
            return;
        }
        let base = self.resolve_base_mode();
        let mode = self.mode;
        if mode == base {
            return;
        }
        if self.enable_status_bar && !self.hud_is_open {
            self.spawn_hud();
        }
        if !is_tooltip_hidden_mode(mode, base) && self.enable_tooltip && !self.tooltip_is_open {
            self.spawn_tooltip(mode);
        }
        self.broadcast_mode_sync();
    }

    /// Broadcast this Daemon instance's current mode to all peers via pipe.
    fn broadcast_mode_sync(&self) {
        pipe_message_to_plugin(
            MessageToPlugin::new("mode_sync")
                .with_payload(format!("{}:{:?}", self.own_client_id, self.mode)),
        );
        // Also broadcast the full ModeInfo so newly spawned Tooltip
        // instances can render keybindings without waiting for the
        // next ModeUpdate event.
        if let Some(mi) = &self.mode_info {
            if let Ok(json) = serde_json::to_string(mi) {
                pipe_message_to_plugin(
                    MessageToPlugin::new("mode_info_sync")
                        .with_payload(format!("{}:{}", self.own_client_id, json)),
                );
            }
        }
    }

    /// Send a single command widget result to HUD instances via pipe.
    fn broadcast_cmd_update(&self, name: &str, stdout: &str) {
        pipe_message_to_plugin(
            MessageToPlugin::new("cmd_update")
                .with_payload(format!("{}:{}:{}", self.own_client_id, name, stdout)),
        );
    }

    /// Send current CWD to HUD instances via pipe.
    fn send_cwd_update(&self) {
        pipe_message_to_plugin(
            MessageToPlugin::new("cwd_update")
                .with_payload(format!(
                    "{}:{}",
                    self.own_client_id,
                    self.cwd.to_string_lossy()
                )),
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
        let mode_changed = self.mode != mode;
        if mode_changed {
            self.mode = mode;
        }
        if self.role == Role::Tooltip {
            // If the new mode is hidden (base/rename/etc), the Daemon has already sent
            // close_tooltip. Don't resize or render — avoids a brief "Normal" flash.
            if is_tooltip_hidden_mode(mode, base) {
                return false;
            }
            if mode_changed {
                let active = self.own_client_id == self.spawned_for_client;
                if active && self.has_permission && !self.tabs.is_empty() {
                    // Resize triggers the host to call render() at the correct
                    // dimensions — no need to render from here.
                    self.resize_tooltip_for_mode();
                }
                // Active: host renders after resize. Inactive: nothing to do.
                return false;
            }
            return false;
        }
        // HUD: always render on mode change.
        mode_changed
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

                // Seed mode and session_name from Daemon's config so
                // the HUD/Tooltip can render immediately without waiting
                // for permission-dependent pipe messages.
                if let Some(mode) = configuration.get("initial_mode").and_then(|s| mode_from_str(s)) {
                    self.mode = mode;
                }
                if let Some(name) = configuration.get("session_name") {
                    self.session_name = name.clone();
                }
                if let Some(cwd) = configuration.get("initial_cwd").filter(|s| !s.is_empty()) {
                    self.cwd = PathBuf::from(cwd);
                }

                // Seed command widget outputs from Daemon's latest results.
                // This allows HUD to render a complete status bar immediately.
                if self.role == Role::Hud {
                    for (key, value) in &configuration {
                        if let Some(name) = key.strip_prefix("cmd_result_") {
                            self.command_outputs.insert(
                                name.to_string(),
                                CommandOutput {
                                    stdout: value.clone(),
                                    exit_code: 0,
                                },
                            );
                        }
                    }
                }

                set_selectable(false);

                request_permission(&[
                    PermissionType::ReadApplicationState,
                    PermissionType::ChangeApplicationState,
                    PermissionType::MessageAndLaunchOtherPlugins,
                ]);
                subscribe(&[
                    EventType::ModeUpdate,
                    EventType::TabUpdate,
                    EventType::Timer,
                    EventType::PermissionRequestResult,
                ]);

                // Initial timer for deferred init fallback.
                set_timeout(0.1);
            }
            Role::Daemon => {
                let ids = get_plugin_ids();
                self.own_client_id = ids.client_id;
                self.cwd = ids.initial_cwd;
                self.spawned_for_client = self.own_client_id;
                self.enable_status_bar =
                    configuration.get("enable_status_bar").map_or(true, |v| v != "false");
                self.enable_tooltip =
                    configuration.get("enable_tooltip").map_or(true, |v| v != "false");
                // Parse HudConfig for command widget definitions.
                // Daemon runs commands on behalf of HUD.
                self.hud_config = HudConfig::from_config(&configuration);
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
                    EventType::Timer,
                    EventType::PermissionRequestResult,
                    EventType::RunCommandResult,
                ]);
                // Deferred init fallback (Timer doesn't require permission).
                set_timeout(0.1);
            }
        }
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(result) => {
                if result == PermissionStatus::Granted {
                    self.has_permission = true;
                    self.run_deferred_init();
                    // Race recovery: ModeUpdate may have arrived before permission
                    // was granted and the spawn was skipped.
                    self.daemon_try_spawn();
                }
                true
            }
            Event::RunCommandResult(exit_code, ref stdout, _stderr, ref context) => {
                if let Some(name) = context.get(CMD_CONTEXT_USER) {
                    let stdout_str = std::str::from_utf8(stdout)
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    self.command_outputs.insert(
                        name.clone(),
                        CommandOutput {
                            stdout: stdout_str.clone(),
                            exit_code: exit_code.unwrap_or(-1),
                        },
                    );
                    // Daemon: forward result to HUD via pipe.
                    if self.role == Role::Daemon && self.hud_is_open {
                        self.broadcast_cmd_update(name, &stdout_str);
                    }
                }
                // Only Daemon receives this (HUD no longer runs commands).
                false
            }
            Event::ModeUpdate(mode_info) => {
                let new_mode = mode_info.mode;
                // Fallback: if PermissionRequestResult arrived before this
                // ModeUpdate, init already ran.  If not, run it now.
                let is_initial = self.mode_info.is_none() && self.has_permission;

                self.session_name = mode_info.session_name.clone().unwrap_or_default();

                // Apply system theme on first ModeUpdate (Styling is now available).
                if self.hud_config.use_system_theme {
                    self.hud_config.apply_system_theme(
                        &mode_info.style.colors,
                        &self.plugin_config,
                    );
                    // Only apply once; subsequent ModeUpdates won't change the theme.
                    self.hud_config.use_system_theme = false;
                }

                self.mode_info = Some(mode_info);

                if is_initial {
                    self.run_deferred_init();
                }

                let base = self.resolve_base_mode();

                match self.role {
                    Role::Hud => {
                        // self.mode is driven exclusively by mode_sync pipe so that
                        // all clients see the same globally active mode.
                    }
                    Role::Tooltip => {
                        // Mode is driven exclusively by mode_sync pipe.
                        // ModeUpdate only updates mode_info (done above).
                        // Resize and render happen in handle_mode_sync_pipe
                        // so the host can process the resize before rendering.
                    }
                    Role::Daemon => {
                        self.mode = new_mode;
                        if self.has_permission {
                            if new_mode == base {
                                // Mode returned to base: close immediately.
                                if self.hud_is_open {
                                    self.close_hud_via_pipe();
                                    self.hud_is_open = false;
                                }
                                if self.tooltip_is_open {
                                    self.close_tooltip_via_pipe();
                                    self.tooltip_is_open = false;
                                }
                            } else {
                                // Non-base: attempt spawn.
                                // daemon_try_spawn guards on tabs/mode_info being ready.
                                self.daemon_try_spawn();
                            }
                            self.broadcast_mode_sync();
                        }
                    }
                }

                let _ = base;
                // Tooltip: don't render on ModeUpdate — mode and resize are
                // driven by mode_sync pipe; host renders after processing resize.
                self.role != Role::Tooltip
            }
            Event::TabUpdate(tabs) => {
                let old_tabs = std::mem::replace(&mut self.tabs, tabs);
                let mut should_render = false;

                // Check if tab list changed (names, count, active state)
                if old_tabs.len() != self.tabs.len()
                    || old_tabs.iter().zip(self.tabs.iter()).any(|(a, b)| {
                        a.active != b.active
                            || a.name != b.name
                            || a.is_sync_panes_active != b.is_sync_panes_active
                            || a.is_fullscreen_active != b.is_fullscreen_active
                    })
                {
                    should_render = true;
                }

                if self.role == Role::Hud || self.role == Role::Tooltip {
                    let is_active_clone = self.own_client_id == self.spawned_for_client;

                    if let Some(active_tab_index) =
                        self.tabs.iter().position(|t| t.active)
                    {
                        let new_idx = active_tab_index + 1;
                        // break_panes_to_tab_with_index requires ChangeApplicationState;
                        // skip until permission is granted to avoid host-side panic.
                        if is_active_clone
                            && self.has_permission
                            && self.active_tab_idx != new_idx
                        {
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

                    if self.role == Role::Tooltip && is_active_clone && self.has_permission {
                        let base = self.resolve_base_mode();
                        if !is_tooltip_hidden_mode(self.mode, base) {
                            self.reposition_tooltip();
                        }
                    }
                }
                // Daemon: tabs just arrived; if permission already granted and we're
                // in non-base mode, this is the spawn recovery for the rare race where
                // PermissionRequestResult fired before the first TabUpdate.
                self.daemon_try_spawn();
                should_render
            }
            Event::Timer(_) => {
                // Fallback: run init if neither PermissionRequestResult
                // nor ModeUpdate triggered it yet.
                if self.has_permission && !self.init_done {
                    self.run_deferred_init();
                }
                match self.role {
                    Role::Hud => {
                        // HUD no longer runs commands (Daemon handles them).
                        // No recurring timer needed; updates arrive via pipe.
                        false
                    }
                    Role::Tooltip => false,
                    Role::Daemon => {
                        set_timeout(1.0);
                        if self.has_permission {
                            // Update CWD and send to HUD if changed.
                            let old_cwd = self.cwd.clone();
                            self.update_cwd_from_focused_pane();
                            if self.cwd != old_cwd && self.hud_is_open {
                                self.send_cwd_update();
                            }
                            // Tick command widget intervals and run due commands.
                            self.tick_command_widgets();
                        }
                        false
                    }
                }
            }
            _ => false,
        }
    }

    fn pipe(&mut self, message: PipeMessage) -> bool {
        let payload = message.payload.as_deref().unwrap_or("");
        match message.name.as_str() {
            "mode_sync" => self.handle_mode_sync_pipe(payload),
            "mode_info_sync" => {
                if self.role == Role::Daemon {
                    return false;
                }
                // Parse "client_id:json" payload.
                if let Some((id_str, json)) = payload.split_once(':') {
                    let client_id: u16 = match id_str.parse() {
                        Ok(id) => id,
                        Err(_) => return false,
                    };
                    if client_id != self.spawned_for_client {
                        return false;
                    }
                    if self.mode_info.is_none() {
                        if let Ok(mi) = serde_json::from_str::<ModeInfo>(json) {
                            // Apply system theme from ModeInfo styling.
                            if self.hud_config.use_system_theme {
                                self.hud_config.apply_system_theme(
                                    &mi.style.colors,
                                    &self.plugin_config,
                                );
                                self.hud_config.use_system_theme = false;
                            }
                            let mi_mode = mi.mode;
                            self.mode_info = Some(mi);
                            // For Tooltip: only trigger render when mode is already in sync
                            // with mode_sync (self.mode == mi_mode). If mode_sync hasn't
                            // arrived yet, it will trigger render (via host after resize).
                            // This prevents a flash of stale keybindings when mode_info_sync
                            // arrives before mode_sync.
                            if self.role != Role::Tooltip || mi_mode == self.mode {
                                return true;
                            }
                        }
                    }
                }
                false
            }
            "request_mode_sync" => {
                // HUD/Tooltip asks for current mode on load.
                // Each Daemon responds; HUD/Tooltip will use only their spawner's reply.
                if self.role == Role::Daemon && self.has_permission {
                    self.broadcast_mode_sync();
                }
                false
            }
            "cmd_update" => {
                // Daemon sends "cmd_update" with payload "client_id:name:value".
                if self.role != Role::Hud {
                    return false;
                }
                let parts: Vec<&str> = payload.splitn(3, ':').collect();
                if parts.len() == 3 {
                    let client_id: u16 = parts[0].parse().unwrap_or(0);
                    if client_id == self.spawned_for_client {
                        self.command_outputs.insert(
                            parts[1].to_string(),
                            CommandOutput {
                                stdout: parts[2].to_string(),
                                exit_code: 0,
                            },
                        );
                        return true;
                    }
                }
                false
            }
            "cwd_update" => {
                // Daemon sends "cwd_update" with payload "client_id:path".
                if self.role != Role::Hud {
                    return false;
                }
                if let Some((id_str, cwd)) = payload.split_once(':') {
                    let client_id: u16 = id_str.parse().unwrap_or(0);
                    if client_id == self.spawned_for_client {
                        self.cwd = PathBuf::from(cwd);
                        return true;
                    }
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

    fn render(&mut self, _rows: usize, cols: usize) {
        match self.role {
            Role::Hud => {
                let c = &self.hud_config;
                let bar_bg = c.resolve_color_with_accent(&c.bar_bg, &c.palette, self.mode);
                let bar_bg_esc = bar_bg.bg();
                let reset = "\x1b[0m";
                let left = self.render_format(&self.hud_config.format_left.clone(), &bar_bg);
                let center_fmt = self.hud_config.format_center.clone();
                let right = self.render_format(&self.hud_config.format_right.clone(), &bar_bg);

                let center = if center_fmt.is_empty() {
                    String::new()
                } else {
                    self.render_format(&center_fmt, &bar_bg)
                };

                let left_visible   = visible_len(&left);
                let center_visible = visible_len(&center);
                let right_visible  = visible_len(&right);

                // Distribute remaining space so that center sits at the absolute
                // middle of the bar.  Guarantee left_gap + right_gap == remaining
                // to prevent the total from exceeding cols on overflow.
                let remaining    = cols.saturating_sub(left_visible + center_visible + right_visible);
                let center_start = (cols.saturating_sub(center_visible)) / 2;
                let left_gap     = center_start.saturating_sub(left_visible).min(remaining);
                let right_gap    = remaining - left_gap;

                // gap_bg: always reset before gap spaces so the last widget's
                // background colour does not bleed into the empty space.
                let gap_bg = format!("{reset}{bar_bg_esc}");

                print!(
                    "{bar_bg_esc}{left}{gap_bg}{lg}{center}{gap_bg}{rg}{right}{reset}",
                    lg = " ".repeat(left_gap),
                    rg = " ".repeat(right_gap),
                );
            }
            Role::Tooltip => {
                self.render_tooltip(_rows, cols);
            }
            Role::Daemon => {}
        }
    }
}
