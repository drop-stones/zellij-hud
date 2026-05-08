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
