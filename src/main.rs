use zellij_tile::prelude::*;

use std::collections::BTreeMap;

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
    tabs: Vec<TabInfo>,
    has_permission: bool,
    hud_is_open: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            is_hud: false,
            mode: InputMode::Locked,
            tabs: Vec::new(),
            has_permission: false,
            hud_is_open: false,
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
        // Get display dimensions from active tab
        let (rows, cols) = self.tabs.iter()
            .find(|t| t.active)
            .map(|t| (t.display_area_rows, t.display_area_columns))
            .unwrap_or((24, 80));

        let height = 2;
        let y = rows.saturating_sub(height);

        FloatingPaneCoordinates::new(
            Some("0".to_string()),
            Some(format!("{}", y)),
            Some(format!("{}", cols)),
            Some(format!("{}", height)),
            Some(true), // pinned
        ).unwrap_or_default()
    }
}

impl ZellijPlugin for State {
    fn load(&mut self, configuration: BTreeMap<String, String>) {
        self.is_hud = configuration.get(CONFIG_IS_HUD).map_or(false, |v| v == "true");

        if self.is_hud {
            // HUD instance: subscribe to mode/tab updates to render and close
            request_permission(&[
                PermissionType::ReadApplicationState,
                PermissionType::ChangeApplicationState,
            ]);
            subscribe(&[
                EventType::ModeUpdate,
                EventType::TabUpdate,
                EventType::PermissionRequestResult,
            ]);
        } else {
            // Daemon instance: hide self and watch for mode changes
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
                        // Daemon: hide immediately
                        hide_self();
                    }
                }
                true
            }
            Event::ModeUpdate(mode_info) => {
                let new_mode = mode_info.mode;

                if self.is_hud {
                    // HUD instance: close when returning to Locked
                    if new_mode == InputMode::Locked {
                        close_self();
                        return false;
                    }
                } else if self.has_permission {
                    // Daemon: spawn HUD on non-Locked, track close on Locked
                    if new_mode != InputMode::Locked && !self.hud_is_open {
                        self.spawn_hud();
                    } else if new_mode == InputMode::Locked {
                        self.hud_is_open = false;
                    }
                }

                self.mode = new_mode;
                true
            }
            Event::TabUpdate(tabs) => {
                self.tabs = tabs;
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
            return; // Daemon renders nothing
        }

        // Bar: mode | tabs
        let mode_str = format!(" {:?} ", self.mode);
        let tab_str: String = self.tabs.iter().map(|t| {
            if t.active {
                format!(" *{} ", t.name)
            } else {
                format!("  {}  ", t.name)
            }
        }).collect();

        let line = format!("{}│{}", mode_str, tab_str);
        let padding = cols.saturating_sub(line.len());
        println!("{}{}", line, " ".repeat(padding));
    }
}
