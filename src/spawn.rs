use zellij_tile::prelude::*;

use crate::tooltip::tooltip_size;
use crate::{State, CONFIG_IS_HUD, CONFIG_IS_TOOLTIP, CONFIG_SPAWNED_FOR_CLIENT};

impl State {
    pub(crate) fn spawn_hud(&mut self) {
        if self.hud_is_open {
            return;
        }
        let initial_tab = self.tabs.iter().position(|t| t.active).map(|i| i + 1).unwrap_or(1);
        let mut config = self.plugin_config.clone();
        config.insert(CONFIG_IS_HUD.to_string(), "true".to_string());
        config.insert(
            CONFIG_SPAWNED_FOR_CLIENT.to_string(),
            self.own_client_id.to_string(),
        );
        config.insert("initial_tab".to_string(), initial_tab.to_string());

        let msg = MessageToPlugin::new("spawn_hud")
            .with_plugin_url("zellij:OWN_URL")
            .with_plugin_config(config)
            .with_floating_pane_coordinates(self.hud_coordinates())
            .new_plugin_instance_should_have_pane_title(String::new());

        pipe_message_to_plugin(msg);
        self.hud_is_open = true;
    }

    /// Spawn a tooltip pane sized for the given active mode.
    /// The tooltip will resize itself dynamically on mode changes.
    pub(crate) fn spawn_tooltip(&mut self, active_mode: InputMode) {
        if self.tooltip_is_open {
            return;
        }

        let (tt_rows, tt_cols) = match &self.mode_info {
            Some(mi) => tooltip_size(mi, active_mode),
            None => return,
        };
        if tt_rows == 0 || tt_cols == 0 {
            return;
        }

        let initial_tab = self.tabs.iter().position(|t| t.active).map(|i| i + 1).unwrap_or(1);
        let mut config = self.plugin_config.clone();
        config.insert(CONFIG_IS_TOOLTIP.to_string(), "true".to_string());
        config.insert(
            CONFIG_SPAWNED_FOR_CLIENT.to_string(),
            self.own_client_id.to_string(),
        );
        config.insert("initial_tab".to_string(), initial_tab.to_string());

        let msg = MessageToPlugin::new("spawn_tooltip")
            .with_plugin_url("zellij:OWN_URL")
            .with_plugin_config(config)
            .with_floating_pane_coordinates(
                self.tooltip_coordinates(tt_rows, tt_cols),
            )
            .new_plugin_instance_should_have_pane_title(String::new());

        pipe_message_to_plugin(msg);
        self.tooltip_is_open = true;
    }

    /// Send a targeted close signal for HUD instances spawned by this client.
    /// The payload carries own_client_id so only the matching HUD closes.
    pub(crate) fn close_hud_via_pipe(&self) {
        pipe_message_to_plugin(
            MessageToPlugin::new("close_hud")
                .with_payload(self.own_client_id.to_string()),
        );
    }

    /// Send a targeted close signal for Tooltip instances spawned by this client.
    pub(crate) fn close_tooltip_via_pipe(&self) {
        pipe_message_to_plugin(
            MessageToPlugin::new("close_tooltip")
                .with_payload(self.own_client_id.to_string()),
        );
    }

    pub(crate) fn hud_coordinates(&self) -> FloatingPaneCoordinates {
        let (rows, cols) = self.display_area();

        let height = 1;
        let y = rows.saturating_sub(height);

        FloatingPaneCoordinates::new(
            Some("0".to_string()),
            Some(format!("{}", y)),
            Some(format!("{}", cols)),
            Some(format!("{}", height)),
            Some(true),
            Some(true),
        )
        .unwrap_or_default()
    }

    fn tooltip_coordinates(
        &self,
        tt_rows: usize,
        tt_cols: usize,
    ) -> FloatingPaneCoordinates {
        let (rows, cols) = self.display_area();

        let hud_height = if self.enable_status_bar { 1 } else { 0 };
        let width = tt_cols.min(cols);
        let height = tt_rows.min(rows.saturating_sub(hud_height));
        let x = cols.saturating_sub(width);
        let y = rows.saturating_sub(hud_height + height);

        FloatingPaneCoordinates::new(
            Some(format!("{}", x)),
            Some(format!("{}", y)),
            Some(format!("{}", width)),
            Some(format!("{}", height)),
            Some(true),
            None,
        )
        .unwrap_or_default()
    }

    fn display_area(&self) -> (usize, usize) {
        self.tabs
            .iter()
            .find(|t| t.active)
            .map(|t| (t.display_area_rows, t.display_area_columns))
            .unwrap_or((24, 80))
    }
}
