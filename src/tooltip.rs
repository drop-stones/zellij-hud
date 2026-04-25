use zellij_tile::prelude::*;

use crate::action_types::ActionType;
use crate::keybinds::{get_actions_for_mode, ModeActions};
use crate::render::visible_len;
use crate::State;

/// Pane frame overhead: 1 top + 1 bottom border row.
const FRAME_ROWS: usize = 2;
/// Pane frame overhead: 1 left + 1 right border col.
const FRAME_COLS: usize = 2;

impl State {
    /// Render vertical which-key style tooltip.
    /// The pane is sized to match the content — no leading/trailing blank rows needed.
    pub(crate) fn render_tooltip(&self, rows: usize, cols: usize) {
        let mode_info = match &self.mode_info {
            Some(m) => m,
            None => {
                for _ in 0..rows {
                    print!("{}", " ".repeat(cols));
                }
                return;
            }
        };

        let ma = get_actions_for_mode(mode_info, self.mode);
        let c = &self.hud_config;
        let border = c.tooltip_border;
        let reset = "\x1b[0m";

        let key_color  = c.resolve_color_with_accent(&c.tooltip_key_color,   &c.palette, self.mode).fg();
        let sep_color  = c.resolve_color_with_accent(&c.tooltip_separator_color, &c.palette, self.mode).fg();
        let desc_color = c.resolve_color_with_accent(&c.tooltip_description_color, &c.palette, self.mode).fg();
        let mode_color = c.resolve_color_with_accent(&c.tooltip_mode_color,   &c.palette, self.mode).fg();
        let bdr_color  = c.resolve_color_with_accent(&c.tooltip_border_color, &c.palette, self.mode).fg();
        let separator  = c.tooltip_separator.clone();

        let inner_cols = if border { cols.saturating_sub(FRAME_COLS) } else { cols };
        let has_common = !ma.common.is_empty();
        let inner_main = rows.saturating_sub(if border { FRAME_ROWS } else { 0 })
                              .saturating_sub(if has_common { 1 } else { 0 });
        let key_width  = ma.actions.iter().map(|a| a.key.len()).max().unwrap_or(0);

        // ── Top border ────────────────────────────────────────────────────────
        if border {
            let title_template = &c.tooltip_title;
            let mode_name = format!("{:?}", self.mode).to_lowercase();
            let title = if title_template.is_empty() {
                String::new()
            } else {
                title_template.replace("{mode}", &mode_name)
            };
            let inner_w = cols.saturating_sub(2);
            let dash_fill = if title.is_empty() {
                "─".repeat(inner_w)
            } else {
                let tv = visible_len(&title);
                let needed = tv + 3;
                if needed >= inner_w {
                    "─".repeat(inner_w)
                } else {
                    format!("─ {} {}", title, "─".repeat(inner_w - needed))
                }
            };
            print!("{bdr_color}┌{dash_fill}┐{reset}");
        }

        // ── Content rows ──────────────────────────────────────────────────────
        let mut row = 0;
        for action in ma.actions.iter().take(inner_main) {
            let key_pad = key_width.saturating_sub(action.key.len());
            let icon = action.action_type.icon();
            let desc = &action.description;
            let d_color = if action.action_type.is_mode_switch() { &mode_color } else { &desc_color };
            let icon_color = match &action.action_type {
                ActionType::SwitchToMode(m) => {
                    c.resolve_color_with_accent("accent", &c.palette, *m).fg()
                }
                _ => action.action_type.icon_color(&c.icon_colors).fg(),
            };
            let content = format!(
                " {key_color}{key}{reset}{pad} {sep_color}{separator}{reset} {icon_color}{icon}{reset} {d_color}{desc}{reset}",
                key = action.key,
                pad = " ".repeat(key_pad),
            );
            let cv = visible_len(&content);
            let pad = inner_cols.saturating_sub(cv);
            if border {
                print!("{bdr_color}│{reset}{content}{}{bdr_color}│{reset}", " ".repeat(pad));
            } else {
                print!("{content}{}", " ".repeat(pad));
            }
            row += 1;
        }
        // Fill any remaining content rows (e.g. pane slightly larger than content)
        while row < inner_main {
            if border {
                print!("{bdr_color}│{reset}{}{bdr_color}│{reset}", " ".repeat(inner_cols));
            } else {
                print!("{}", " ".repeat(cols));
            }
            row += 1;
        }

        // ── Common (back/exit) keys ───────────────────────────────────────────
        if has_common {
            let icons: Vec<&str> = ma.common.iter().map(|c| c.icon).collect();
            let desc = &ma.common[0].description;
            let content = format!(
                "{key_color}{icons}{reset} {sep_color}{desc}{reset}",
                icons = icons.join(" "),
            );
            let cv = visible_len(&content);
            let lpad = inner_cols.saturating_sub(cv) / 2;
            let rpad = inner_cols.saturating_sub(cv + lpad);
            if border {
                print!("{bdr_color}│{reset}{}{content}{}{bdr_color}│{reset}",
                    " ".repeat(lpad), " ".repeat(rpad));
            } else {
                print!("{}{content}{}", " ".repeat(lpad), " ".repeat(rpad));
            }
        }

        // ── Bottom border ─────────────────────────────────────────────────────
        if border {
            print!("{bdr_color}└{}┘{reset}", "─".repeat(cols.saturating_sub(2)));
        }
    }

    /// Resize the tooltip pane to the current mode's content size.
    /// Always calls the API so the host re-renders at the correct dimensions.
    pub(crate) fn resize_tooltip_for_mode(&mut self) {
        let (plugin_id, mode_info) = match (self.own_plugin_id, &self.mode_info) {
            (Some(id), Some(mi)) => (id, mi),
            _ => return,
        };
        if self.tabs.is_empty() {
            return;
        }
        let border = self.hud_config.tooltip_border;
        let (height, width) = tooltip_size(mode_info, self.mode, border);
        if height == 0 || width == 0 {
            return;
        }
        self.last_tooltip_size = (height, width);
        let coords = tooltip_coordinates_at_size(
            height, width, &self.tabs,
            self.hud_config.enable_status_bar,
            &self.hud_config.tooltip_position,
        );
        change_floating_panes_coordinates(vec![(PaneId::Plugin(plugin_id), coords)]);
    }

    /// Reposition the tooltip at current content size (for TabUpdate / display area change).
    pub(crate) fn reposition_tooltip(&self) {
        let (plugin_id, mode_info) = match (self.own_plugin_id, &self.mode_info) {
            (Some(id), Some(mi)) => (id, mi),
            _ => return,
        };
        if self.tabs.is_empty() {
            return;
        }
        let border = self.hud_config.tooltip_border;
        let (height, width) = tooltip_size(mode_info, self.mode, border);
        if height == 0 || width == 0 {
            return;
        }
        let coords = tooltip_coordinates_at_size(
            height, width, &self.tabs,
            self.hud_config.enable_status_bar,
            &self.hud_config.tooltip_position,
        );
        change_floating_panes_coordinates(vec![(PaneId::Plugin(plugin_id), coords)]);
    }

}

/// Calculate tooltip pane size for a specific mode (content + manual border if enabled).
pub(crate) fn tooltip_size(mode_info: &ModeInfo, mode: InputMode, border: bool) -> (usize, usize) {
    let ma = get_actions_for_mode(mode_info, mode);
    if ma.actions.is_empty() && ma.common.is_empty() {
        return (0, 0);
    }
    tooltip_dimensions(&ma, border)
}

/// Compute (height, width) for a ModeActions including manual border overhead.
fn tooltip_dimensions(ma: &ModeActions, border: bool) -> (usize, usize) {
    let key_width = ma.actions.iter().map(|a| a.key.len()).max().unwrap_or(0);
    let desc_width = ma
        .actions
        .iter()
        .map(|a| visible_len(a.action_type.icon()) + 1 + a.description.len())
        .max()
        .unwrap_or(0);

    // " key  ➜ icon desc"  (leading space + key + 3 separators + desc)
    let main_width = 1 + key_width + 3 + desc_width;

    let common_width = if ma.common.is_empty() {
        0
    } else {
        let icons_w = ma.common.len();
        let seps_w  = ma.common.len().saturating_sub(1);
        icons_w + seps_w + 1 + ma.common[0].description.len()
    };

    let content_width  = main_width.max(common_width);
    let content_height = ma.actions.len() + if ma.common.is_empty() { 0 } else { 1 };

    let frame_r = if border { FRAME_ROWS } else { 0 };
    let frame_c = if border { FRAME_COLS } else { 0 };
    (content_height + frame_r, content_width + frame_c)
}

/// Build FloatingPaneCoordinates for a tooltip given explicit dimensions.
/// The pane is always borderless (border is drawn manually inside render_tooltip).
fn tooltip_coordinates_at_size(
    tt_rows: usize,
    tt_cols: usize,
    tabs: &[TabInfo],
    status_bar_enabled: bool,
    position: &str,
) -> FloatingPaneCoordinates {
    let (rows, cols) = tabs
        .iter()
        .find(|t| t.active)
        .map(|t| (t.display_area_rows, t.display_area_columns))
        .unwrap_or((24, 80));

    let hud_height = if status_bar_enabled { 1 } else { 0 };
    let w = tt_cols.min(cols);
    let h = tt_rows.min(rows.saturating_sub(hud_height));

    let (x, y) = match position {
        "bottom-left" => (0, rows.saturating_sub(hud_height + h)),
        "top-right"   => (cols.saturating_sub(w), 0),
        "top-left"    => (0, 0),
        _             => (cols.saturating_sub(w), rows.saturating_sub(hud_height + h)),
    };

    FloatingPaneCoordinates::new(
        Some(format!("{x}")),
        Some(format!("{y}")),
        Some(format!("{w}")),
        Some(format!("{h}")),
        Some(true),
        Some(true), // always borderless: manual border drawn in render_tooltip
    )
    .unwrap_or_default()
}

