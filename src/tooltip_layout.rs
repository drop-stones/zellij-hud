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

/// Modes where the tooltip should not be shown (base mode + text input modes).
///
/// Text input modes (RenamePane, RenameTab, EnterSearch) are excluded because
/// the keybinding hints would obscure the user's typing target.
pub fn is_tooltip_hidden_mode(mode: InputMode, base_mode: InputMode) -> bool {
    mode == base_mode
        || matches!(
            mode,
            InputMode::RenamePane | InputMode::RenameTab | InputMode::EnterSearch
        )
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action_types::ActionType;
    use crate::keybinds::{CommonKey, KeyAction};

    fn action(key: &str, desc: &str, icon: ActionType) -> KeyAction {
        KeyAction {
            key: key.to_string(),
            description: desc.to_string(),
            action_type: icon,
        }
    }

    fn common_keys(desc: &str, icons: &[&'static str]) -> Vec<CommonKey> {
        icons
            .iter()
            .map(|i| CommonKey {
                icon: i,
                description: desc.to_string(),
            })
            .collect()
    }

    // ---- tooltip_dimensions ----

    #[test]
    fn tooltip_dimensions_basic_with_no_common() {
        let ma = ModeActions {
            actions: vec![
                action("h", "left", ActionType::MoveFocusLeft),
                action("ll", "right", ActionType::MoveFocusLeft),
            ],
            common: vec![],
        };
        // key_width = 2 ("ll"), desc_width = visible_len(icon)+1+max("left"|"right".len())
        // ActionType::MoveFocusLeft icon visible_len = 1, max desc = 5 ("right")
        // main_width = 1 + 2 + 3 + (1+1+5) = 13
        // content_width = 13 + 1 right margin = 14
        // content_height = 2 actions, no common → 2
        let (h, w) = tooltip_dimensions(&ma, false);
        assert_eq!((h, w), (2, 14));
    }

    #[test]
    fn tooltip_dimensions_includes_common_row() {
        let ma = ModeActions {
            actions: vec![action("h", "x", ActionType::MoveFocusLeft)],
            common: common_keys("exit", &["x"]),
        };
        // 1 action + 1 common row = height 2.
        let (h, _w) = tooltip_dimensions(&ma, false);
        assert_eq!(h, 2);
    }

    #[test]
    fn tooltip_dimensions_picks_max_of_main_and_common() {
        // common is wider than the main action grid → width follows common.
        let ma = ModeActions {
            actions: vec![action("h", "x", ActionType::MoveFocusLeft)],
            common: common_keys("very long description of common", &["x"]),
        };
        let (_h, w) = tooltip_dimensions(&ma, false);
        // common_width = 1 (icon) + 0 (no sep) + 1 (space) + 31 (desc) = 33
        // main_width = 1 + 1 + 3 + (1+1+1) = 8
        // → width = max(8, 33) + 1 right margin = 34
        assert_eq!(w, 34);
    }

    #[test]
    fn tooltip_dimensions_adds_frame_with_border() {
        let ma = ModeActions {
            actions: vec![action("h", "left", ActionType::MoveFocusLeft)],
            common: vec![],
        };
        let (h_no, w_no) = tooltip_dimensions(&ma, false);
        let (h_b,  w_b)  = tooltip_dimensions(&ma, true);
        assert_eq!(h_b, h_no + FRAME_ROWS);
        assert_eq!(w_b, w_no + FRAME_COLS);
    }

    // ---- tooltip_position_size ----

    #[test]
    fn tooltip_position_size_bottom_right_is_default() {
        let (x, y, w, h) = tooltip_position_size(5, 20, 24, 80, false, "bottom-right");
        assert_eq!((x, y, w, h), (60, 19, 20, 5));
        // Unrecognised position falls back to bottom-right too.
        assert_eq!(
            tooltip_position_size(5, 20, 24, 80, false, "garbage"),
            (60, 19, 20, 5),
        );
    }

    #[test]
    fn tooltip_position_size_corners() {
        assert_eq!(
            tooltip_position_size(5, 20, 24, 80, false, "bottom-left"),
            (0, 19, 20, 5),
        );
        assert_eq!(
            tooltip_position_size(5, 20, 24, 80, false, "top-right"),
            (60, 0, 20, 5),
        );
        assert_eq!(
            tooltip_position_size(5, 20, 24, 80, false, "top-left"),
            (0, 0, 20, 5),
        );
    }

    #[test]
    fn tooltip_position_size_status_bar_reserves_a_row_at_bottom() {
        // Status bar enabled → bottom-anchored y shifts up by 1, and the
        // available height shrinks by 1.
        let (_x, y_off, _w, _h) =
            tooltip_position_size(5, 20, 24, 80, true, "bottom-right");
        let (_x2, y_on, _w2, _h2) =
            tooltip_position_size(5, 20, 24, 80, false, "bottom-right");
        assert_eq!(y_on - y_off, 1);
    }

    #[test]
    fn tooltip_position_size_clamps_to_display_area() {
        // Tooltip larger than the screen → clamped to area.
        let (_x, _y, w, h) = tooltip_position_size(100, 200, 24, 80, false, "top-left");
        assert_eq!((w, h), (80, 24));
        // With status bar, height clamp accounts for the reserved row.
        let (_x, _y, _w, h) = tooltip_position_size(100, 200, 24, 80, true, "top-left");
        assert_eq!(h, 23);
    }

    #[test]
    fn tooltip_position_size_handles_tooltip_taller_than_area_at_bottom() {
        // saturating_sub on y guards against underflow when the tooltip
        // wouldn't fit above the status bar.
        let (_x, y, _w, _h) = tooltip_position_size(50, 10, 24, 80, true, "bottom-right");
        assert_eq!(y, 0);
    }

    // ---- tooltip_size (integration of get_actions_for_mode + tooltip_dimensions) ----

    #[test]
    fn tooltip_size_returns_zero_for_mode_with_no_actions() {
        // Default ModeInfo has no keybinds → empty actions → (0, 0).
        let mi = ModeInfo::default();
        assert_eq!(tooltip_size(&mi, InputMode::Normal, true), (0, 0));
    }

    // ---- is_tooltip_hidden_mode ----

    #[test]
    fn is_tooltip_hidden_mode_hides_when_in_base_mode() {
        // Base mode varies per user (auto-detected from keybinds), so the
        // function is parameterised — anything that equals the base mode hides.
        assert!(is_tooltip_hidden_mode(InputMode::Normal, InputMode::Normal));
        assert!(is_tooltip_hidden_mode(InputMode::Locked, InputMode::Locked));
    }

    #[test]
    fn is_tooltip_hidden_mode_hides_text_input_modes_regardless_of_base() {
        // Tooltip would obscure the user's typing target in these modes.
        for text_mode in [
            InputMode::RenamePane,
            InputMode::RenameTab,
            InputMode::EnterSearch,
        ] {
            assert!(
                is_tooltip_hidden_mode(text_mode, InputMode::Normal),
                "expected {text_mode:?} to be hidden",
            );
        }
    }

    #[test]
    fn is_tooltip_hidden_mode_shows_normal_modes() {
        // Action modes show the tooltip when they're not the base.
        for mode in [
            InputMode::Pane,
            InputMode::Tab,
            InputMode::Resize,
            InputMode::Move,
            InputMode::Scroll,
            InputMode::Search,
            InputMode::Session,
            InputMode::Prompt,
            InputMode::Tmux,
        ] {
            assert!(
                !is_tooltip_hidden_mode(mode, InputMode::Normal),
                "expected {mode:?} to be visible",
            );
        }
    }
}
