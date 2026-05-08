//! Pure layout math for the tooltip pane.
//!
//! The tooltip's pane size and screen position are derived purely from the
//! current mode's actions, the active tab's display area, and a few config
//! flags. Splitting these helpers out of `crate::tooltip` (bin) lets us cover
//! them with host tests; the bin-side wrapper stitches in zellij-tile shim
//! types (`TabInfo`, `FloatingPaneCoordinates`) and shim calls.

use zellij_tile::prelude::{InputMode, ModeInfo};

use crate::keybinds::{get_actions_for_mode, ModeActions};
use crate::text::visible_len;

/// Pane frame overhead: 1 top + 1 bottom border row.
pub const FRAME_ROWS: usize = 2;
/// Pane frame overhead: 1 left + 1 right border col.
pub const FRAME_COLS: usize = 2;

/// Calculate tooltip pane size for a specific mode (content + manual border if enabled).
/// Returns (0, 0) when the mode has no actions to display.
pub fn tooltip_size(mode_info: &ModeInfo, mode: InputMode, border: bool) -> (usize, usize) {
    let ma = get_actions_for_mode(mode_info, mode);
    if ma.actions.is_empty() && ma.common.is_empty() {
        return (0, 0);
    }
    tooltip_dimensions(&ma, border)
}

/// Compute (height, width) for a ModeActions including manual border overhead.
pub fn tooltip_dimensions(ma: &ModeActions, border: bool) -> (usize, usize) {
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

    let content_width  = main_width.max(common_width) + 1; // +1 right margin
    let content_height = ma.actions.len() + if ma.common.is_empty() { 0 } else { 1 };

    let frame_r = if border { FRAME_ROWS } else { 0 };
    let frame_c = if border { FRAME_COLS } else { 0 };
    (content_height + frame_r, content_width + frame_c)
}

/// Compute floating-pane (x, y, width, height) for a tooltip of the requested
/// size, given the display area and which corner to anchor to.
///
/// `position` is one of `"bottom-right"` (default), `"bottom-left"`,
/// `"top-right"`, `"top-left"`. Unrecognized values fall back to bottom-right.
/// `status_bar_enabled` reserves one row at the bottom for the HUD.
pub fn tooltip_position_size(
    tt_rows: usize,
    tt_cols: usize,
    area_rows: usize,
    area_cols: usize,
    status_bar_enabled: bool,
    position: &str,
) -> (usize, usize, usize, usize) {
    let hud_height = if status_bar_enabled { 1 } else { 0 };
    let w = tt_cols.min(area_cols);
    let h = tt_rows.min(area_rows.saturating_sub(hud_height));

    let (x, y) = match position {
        "bottom-left" => (0, area_rows.saturating_sub(hud_height + h)),
        "top-right"   => (area_cols.saturating_sub(w), 0),
        "top-left"    => (0, 0),
        _             => (area_cols.saturating_sub(w), area_rows.saturating_sub(hud_height + h)),
    };
    (x, y, w, h)
}
