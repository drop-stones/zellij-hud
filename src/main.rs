use zellij_tile::prelude::*;

use std::collections::BTreeMap;

/// PoC: On-demand floating status bar for zellij.
///
/// - Hidden in Locked mode (zero footprint)
/// - Appears as a floating bar at the bottom on mode change
/// - Shows current mode + tab list
#[derive(Default)]
struct State {
    mode: InputMode,
    tabs: Vec<TabInfo>,
    is_hidden: bool,
    has_permission: bool,
    plugin_id: Option<u32>,
    display_rows: usize,
    display_cols: usize,
}

register_plugin!(State);

impl State {
    /// Reposition the floating pane to the bottom of the screen
    fn reposition_to_bottom(&self) {
        if let Some(id) = self.plugin_id {
            let height = 3;
            let width_pct = "100%".to_string();
            // Position at bottom: y = display_rows - height - 1 (for border)
            let y = if self.display_rows > height + 1 {
                self.display_rows - height - 1
            } else {
                0
            };
            if let Some(coords) = FloatingPaneCoordinates::new(
                Some("0".to_string()),          // x: left edge
                Some(format!("{}", y)),          // y: bottom
                Some(width_pct),                // width: full
                Some(format!("{}", height)),     // height: 3 rows
                Some(true),                     // pinned: don't lose focus
            ) {
                change_floating_panes_coordinates(vec![
                    (PaneId::Plugin(id), coords)
                ]);
            }
        }
    }
}

impl ZellijPlugin for State {
    fn load(&mut self, _configuration: BTreeMap<String, String>) {
        request_permission(&[
            PermissionType::ReadApplicationState,
            PermissionType::ChangeApplicationState,
        ]);
        subscribe(&[
            EventType::ModeUpdate,
            EventType::TabUpdate,
            EventType::PermissionRequestResult,
        ]);
        self.is_hidden = false; // Start visible until permission granted
    }

    fn update(&mut self, event: Event) -> bool {
        match event {
            Event::PermissionRequestResult(result) => {
                if result == PermissionStatus::Granted {
                    self.has_permission = true;
                    // Get our plugin ID for repositioning
                    let ids = get_plugin_ids();
                    self.plugin_id = Some(ids.plugin_id);
                    // Hide initially
                    hide_self();
                    self.is_hidden = true;
                }
                true
            }
            Event::ModeUpdate(mode_info) => {
                let new_mode = mode_info.mode;

                if self.has_permission {
                    if new_mode != InputMode::Locked && self.is_hidden {
                        show_self(true);
                        self.is_hidden = false;
                        self.reposition_to_bottom();
                    } else if new_mode == InputMode::Locked && !self.is_hidden {
                        hide_self();
                        self.is_hidden = true;
                    }
                }

                self.mode = new_mode;
                true
            }
            Event::TabUpdate(tabs) => {
                // Use tab info to get display dimensions
                if let Some(tab) = tabs.iter().find(|t| t.active) {
                    self.display_rows = tab.display_area_rows;
                    self.display_cols = tab.display_area_columns;
                }
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
        // Line 1: mode + tabs (bar style)
        let mode_str = format!(" {:?} ", self.mode);
        let tab_str: String = self.tabs.iter().map(|t| {
            if t.active {
                format!(" *{} ", t.name)
            } else {
                format!("  {}  ", t.name)
            }
        }).collect();

        let line = format!("{}│{}", mode_str, tab_str);
        // Pad to fill width
        let padding = cols.saturating_sub(line.len());
        println!("{}{}", line, " ".repeat(padding));
    }
}
