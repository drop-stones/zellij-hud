//! Pure decision functions extracted from `State`'s event handlers.
//!
//! Each `decide_*` function takes the bits of state that drive a behaviour
//! choice and returns a `*Decision` struct describing what the bin-side
//! caller should do next: which fields to mutate, which side-effect helpers
//! to invoke, and what to return from the event handler.

use zellij_tile::prelude::InputMode;

use crate::tooltip_layout::is_tooltip_hidden_mode;

/// Outcome of a mode_sync pipe message after the envelope has been parsed
/// and the client_id has been confirmed.
#[derive(Debug, PartialEq, Eq)]
pub struct ModeSyncDecision {
    /// `Some(mode)` → the caller should set `self.mode = mode`.
    /// `None` → leave `self.mode` untouched (e.g. transitioning to base mode,
    /// where rendering the base label briefly would flash before close).
    pub new_self_mode: Option<InputMode>,
    /// `true` → the caller (Tooltip role) should set
    /// `self.tooltip_needs_resize = true`. Always false for HUD.
    pub tooltip_needs_resize: bool,
    /// Return value for `pipe()` — whether to trigger a render.
    pub should_render: bool,
}

/// Decide what to do for a mode_sync message, given the current state of a
/// HUD or Tooltip instance.
///
/// `is_tooltip = true` selects the tooltip-specific branches (hidden-mode
/// gating, resize flagging). Daemons are pre-filtered by the bin-side
/// handler and never reach here.
pub fn decide_mode_sync(
    is_tooltip: bool,
    current_mode: InputMode,
    payload_mode: InputMode,
    base_mode: InputMode,
) -> ModeSyncDecision {
    let mode_changed = current_mode != payload_mode;

    // Skip self.mode update when transitioning to base mode: the Daemon's
    // close_{hud,tooltip} pipe will close this instance shortly, and
    // rendering base-mode content before the close causes a flash.
    let new_self_mode = if mode_changed && payload_mode != base_mode {
        Some(payload_mode)
    } else {
        None
    };

    if is_tooltip {
        // Hidden modes (rename / enter_search): skip render so the user
        // doesn't see a flash of the wrong keybinding hints at old
        // dimensions before the Daemon closes us.
        if is_tooltip_hidden_mode(payload_mode, base_mode) {
            return ModeSyncDecision {
                new_self_mode,
                tooltip_needs_resize: false,
                should_render: false,
            };
        }
        return ModeSyncDecision {
            new_self_mode,
            tooltip_needs_resize: mode_changed,
            should_render: mode_changed,
        };
    }

    // HUD: render on mode change, except when transitioning to base mode
    // (avoid a "LOCKED" flash before close).
    ModeSyncDecision {
        new_self_mode,
        tooltip_needs_resize: false,
        should_render: mode_changed && payload_mode != base_mode,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- HUD ----

    #[test]
    fn hud_unchanged_mode_yields_no_render_no_update() {
        // current == payload → no-op for both fields and the return value.
        let d = decide_mode_sync(false, InputMode::Pane, InputMode::Pane, InputMode::Normal);
        assert_eq!(d.new_self_mode, None);
        assert!(!d.tooltip_needs_resize);
        assert!(!d.should_render);
    }

    #[test]
    fn hud_normal_mode_change_updates_self_and_renders() {
        // Pane → Tab while base=Normal: classic transition.
        let d = decide_mode_sync(false, InputMode::Pane, InputMode::Tab, InputMode::Normal);
        assert_eq!(d.new_self_mode, Some(InputMode::Tab));
        assert!(!d.tooltip_needs_resize);
        assert!(d.should_render);
    }

    #[test]
    fn hud_transition_to_base_does_not_update_or_render() {
        // Avoid a "Normal" flash before the daemon's close_hud pipe arrives.
        let d = decide_mode_sync(false, InputMode::Pane, InputMode::Normal, InputMode::Normal);
        assert_eq!(d.new_self_mode, None);
        assert!(!d.should_render);
    }

    // ---- Tooltip ----

    #[test]
    fn tooltip_unchanged_mode_yields_no_resize_no_render() {
        let d = decide_mode_sync(true, InputMode::Pane, InputMode::Pane, InputMode::Normal);
        assert_eq!(d.new_self_mode, None);
        assert!(!d.tooltip_needs_resize);
        assert!(!d.should_render);
    }

    #[test]
    fn tooltip_normal_mode_change_resizes_and_renders() {
        let d = decide_mode_sync(true, InputMode::Pane, InputMode::Tab, InputMode::Normal);
        assert_eq!(d.new_self_mode, Some(InputMode::Tab));
        assert!(d.tooltip_needs_resize);
        assert!(d.should_render);
    }

    #[test]
    fn tooltip_transition_to_base_does_not_update_but_does_not_render() {
        // Tooltip behaves like HUD here: skip the update so we don't render
        // the base label briefly before close.
        let d = decide_mode_sync(true, InputMode::Pane, InputMode::Normal, InputMode::Normal);
        assert_eq!(d.new_self_mode, None);
        assert!(!d.should_render);
    }

    #[test]
    fn tooltip_hidden_mode_skips_render_even_on_change() {
        // RenamePane / RenameTab / EnterSearch: keybinding hints would
        // obscure typing target. Daemon will close us shortly, so skip
        // render to avoid an old-dimension flash.
        for hidden in [
            InputMode::RenamePane,
            InputMode::RenameTab,
            InputMode::EnterSearch,
        ] {
            let d = decide_mode_sync(true, InputMode::Pane, hidden, InputMode::Normal);
            // Mode update still happens (we'll be closing, but state is
            // recorded for any subsequent decision); render is suppressed.
            assert_eq!(d.new_self_mode, Some(hidden), "hidden={hidden:?}");
            assert!(!d.tooltip_needs_resize, "hidden={hidden:?}");
            assert!(!d.should_render, "hidden={hidden:?}");
        }
    }

    #[test]
    fn hud_does_not_skip_render_for_text_input_modes() {
        // The hidden-mode skip is a Tooltip-only concern. The HUD continues
        // to render across rename/search transitions because its content
        // (mode label) is still meaningful.
        let d = decide_mode_sync(false, InputMode::Pane, InputMode::RenamePane, InputMode::Normal);
        assert_eq!(d.new_self_mode, Some(InputMode::RenamePane));
        assert!(d.should_render);
    }
}
