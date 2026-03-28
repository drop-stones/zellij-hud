use zellij_tile::prelude::*;

use crate::action_types::ActionType;
use crate::keybinds::{get_actions_for_mode, ModeActions};
use crate::render::visible_len;
use crate::State;

/// Floating pane frame overhead (top + bottom border).
const FRAME_ROWS: usize = 2;
/// Floating pane frame overhead (left + right border).
const FRAME_COLS: usize = 2;

impl State {
    /// Render vertical which-key style tooltip.
    pub(crate) fn render_tooltip(&self, rows: usize, cols: usize) {
        let mode_info = match &self.mode_info {
            Some(m) => m,
            None => return,
        };

        let ma = get_actions_for_mode(mode_info, self.mode);

        let c = &self.hud_config;
        let reset = "\x1b[0m";
        let key_color = c.resolve_color_with_accent(&c.tooltip_key_color, &c.palette, self.mode).fg();
        let sep_color = c.resolve_color_with_accent(&c.tooltip_separator_color, &c.palette, self.mode).fg();
        let desc_color = c.resolve_color_with_accent(&c.tooltip_description_color, &c.palette, self.mode).fg();
        let mode_color = c.resolve_color_with_accent(&c.tooltip_mode_color, &c.palette, self.mode).fg();
        let separator = &c.tooltip_separator;

        let has_common = !ma.common.is_empty();
        let main_rows = if has_common {
            rows.saturating_sub(1)
        } else {
            rows
        };

        let key_width = ma
            .actions
            .iter()
            .map(|a| a.key.len())
            .max()
            .unwrap_or(0);

        let mut row = 0;
        for action in ma.actions.iter().take(main_rows) {
            let key_pad = key_width.saturating_sub(action.key.len());
            let icon = action.action_type.icon();
            let desc = &action.description;
            let d_color = if action.action_type.is_mode_switch() {
                &mode_color
            } else {
                &desc_color
            };

            // Mode switch icon uses per-mode accent color
            let icon_color = match &action.action_type {
                ActionType::SwitchToMode(m) => {
                    c.resolve_color_with_accent("accent", &c.palette, *m).fg()
                }
                _ => action.action_type.icon_color(&c.icon_colors).fg(),
            };

            let line = format!(
                " {key_color}{key}{reset}{pad} {sep_color}{separator}{reset} {icon_color}{icon}{reset} {d_color}{desc}{reset}",
                key = action.key,
                pad = " ".repeat(key_pad),
            );
            let line_visible = visible_len(&line);
            let trailing = cols.saturating_sub(line_visible);
            print!("{line}{}", " ".repeat(trailing));
            row += 1;
        }

        // Fill remaining main rows
        while row < main_rows {
            print!("{}", " ".repeat(cols));
            row += 1;
        }

        // Render common (back/exit) keys centered at bottom
        if has_common && row < rows {
            let dim = &sep_color;
            // Collect icons and use the shared description
            let icons: Vec<&str> =
                ma.common.iter().map(|c| c.icon).collect();
            let desc = &ma.common[0].description;
            let content = format!(
                "{key_color}{icons}{reset} {dim}{desc}{reset}",
                icons = icons.join(" "),
            );
            let content_visible = visible_len(&content);
            let left_pad = cols.saturating_sub(content_visible) / 2;
            let right_pad =
                cols.saturating_sub(content_visible + left_pad);
            print!(
                "{}{}{}",
                " ".repeat(left_pad),
                content,
                " ".repeat(right_pad),
            );
        }
    }

    /// Resize the tooltip pane to fit the current mode's keybindings.
    pub(crate) fn resize_tooltip_for_mode(&self) {
        let (plugin_id, mode_info) =
            match (self.own_plugin_id, &self.mode_info) {
                (Some(id), Some(mi)) => (id, mi),
                _ => return,
            };

        if self.tabs.is_empty() {
            return;
        }

        let ma = get_actions_for_mode(mode_info, self.mode);
        if ma.actions.is_empty() && ma.common.is_empty() {
            return;
        }

        let border = self.hud_config.tooltip_border;
        let coords = tooltip_coordinates(
            &ma,
            &self.tabs,
            self.hud_config.enable_status_bar,
            &self.hud_config.tooltip_position,
            border,
        );
        change_floating_panes_coordinates(vec![(
            PaneId::Plugin(plugin_id),
            coords,
        )]);
    }

    /// Update the tooltip pane title to show the current mode.
    pub(crate) fn update_tooltip_title(&self) {
        let plugin_id = match self.own_plugin_id {
            Some(id) => id,
            None => return,
        };

        let template = &self.hud_config.tooltip_title;
        if template.is_empty() {
            rename_plugin_pane(plugin_id, "");
            return;
        }

        let mode_name = format!("{:?}", self.mode).to_lowercase();
        let title = template.replace("{mode}", &mode_name);
        rename_plugin_pane(plugin_id, &title);
    }
}

/// Calculate tooltip pane size for the initial spawn.
pub(crate) fn tooltip_size(mode_info: &ModeInfo, mode: InputMode, border: bool) -> (usize, usize) {
    let ma = get_actions_for_mode(mode_info, mode);
    if ma.actions.is_empty() && ma.common.is_empty() {
        return (0, 0);
    }
    tooltip_dimensions(&ma, border)
}

/// Compute (height, width) including frame for a ModeActions.
fn tooltip_dimensions(ma: &ModeActions, border: bool) -> (usize, usize) {
    let key_width = ma
        .actions
        .iter()
        .map(|a| a.key.len())
        .max()
        .unwrap_or(0);
    let desc_width = ma
        .actions
        .iter()
        .map(|a| {
            let icon = a.action_type.icon();
            visible_len(icon) + 1 + a.description.len()
        })
        .max()
        .unwrap_or(0);

    // " key  ➜ icon desc"
    let main_width = 1 + key_width + 3 + desc_width;

    // Common line: "icon icon desc"
    let common_width = if ma.common.is_empty() {
        0
    } else {
        let icons_width: usize = ma.common.len(); // each icon is 1 visible char
        let sep_width = ma.common.len().saturating_sub(1); // spaces between icons
        let desc_len = ma.common[0].description.len();
        icons_width + sep_width + 1 + desc_len
    };

    let content_width = main_width.max(common_width);
    let content_height =
        ma.actions.len() + if ma.common.is_empty() { 0 } else { 1 };

    let frame = if border { FRAME_ROWS } else { 0 };
    let frame_cols = if border { FRAME_COLS } else { 0 };
    (content_height + frame, content_width + frame_cols)
}

/// Build FloatingPaneCoordinates for a tooltip.
fn tooltip_coordinates(
    ma: &ModeActions,
    tabs: &[TabInfo],
    status_bar_enabled: bool,
    position: &str,
    border: bool,
) -> FloatingPaneCoordinates {
    let (rows, cols) = tabs
        .iter()
        .find(|t| t.active)
        .map(|t| (t.display_area_rows, t.display_area_columns))
        .unwrap_or((24, 80));

    let (height, width) = tooltip_dimensions(ma, border);
    let hud_height = if status_bar_enabled { 1 } else { 0 };
    let w = width.min(cols);
    let h = height.min(rows.saturating_sub(hud_height));

    let (x, y) = match position {
        "bottom-left" => {
            (0, rows.saturating_sub(hud_height + h))
        }
        "top-right" => {
            (cols.saturating_sub(w), 0)
        }
        "top-left" => {
            (0, 0)
        }
        // "bottom-right" (default)
        _ => {
            (cols.saturating_sub(w), rows.saturating_sub(hud_height + h))
        }
    };

    FloatingPaneCoordinates::new(
        Some(format!("{}", x)),
        Some(format!("{}", y)),
        Some(format!("{}", w)),
        Some(format!("{}", h)),
        Some(true),
        if border { None } else { Some(true) },
    )
    .unwrap_or_default()
}
