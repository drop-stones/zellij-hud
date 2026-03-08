use zellij_tile::prelude::*;

use crate::{State, CONFIG_IS_HUD};

impl State {
    pub(crate) fn spawn_hud(&mut self) {
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

    pub(crate) fn hud_coordinates(&self) -> FloatingPaneCoordinates {
        let (rows, cols) = self
            .tabs
            .iter()
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
        )
        .unwrap_or_default()
    }
}
