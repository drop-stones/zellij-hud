//! Pure decision functions extracted from `State`'s event handlers.
//!
//! Each `decide_*` function takes the bits of state that drive a behaviour
//! choice and returns a `*Decision` struct describing what the bin-side
//! caller should do next: which fields to mutate, which side-effect helpers
//! to invoke, and what to return from the event handler.

use zellij_tile::prelude::InputMode;

use crate::role::Role;
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
        // Hidden modes (base mode + rename / enter_search per
        // `is_tooltip_hidden_mode`): skip render so the user doesn't see a
        // flash of the wrong keybinding hints at old dimensions before the
        // Daemon closes us.
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

/// Outcome of a `ModeUpdate` event for any role.
///
/// Each bool corresponds to one side-effect the bin-side caller should
/// invoke; `new_self_mode` carries an `Option` because some role/state
/// combinations skip the mode update to avoid a flash before close.
#[derive(Debug, PartialEq, Eq)]
pub struct ModeUpdateDecision {
    pub new_self_mode: Option<InputMode>,
    pub close_hud: bool,
    pub close_tooltip: bool,
    pub daemon_try_spawn: bool,
    pub broadcast_mode_sync: bool,
    pub tooltip_needs_resize: bool,
    pub should_render: bool,
}

impl ModeUpdateDecision {
    fn noop() -> Self {
        Self {
            new_self_mode: None,
            close_hud: false,
            close_tooltip: false,
            daemon_try_spawn: false,
            broadcast_mode_sync: false,
            tooltip_needs_resize: false,
            should_render: false,
        }
    }
}

/// Decide what to do for a `ModeUpdate` event, given the role-specific bits
/// of state. Daemon owns the lifecycle (close/spawn/broadcast); HUD and
/// Tooltip just react to their spawning client's mode if they're the active
/// clone and skip rendering across base-mode transitions to avoid a flash.
///
/// `is_initial` is the "this is the first ModeUpdate after spawn" flag —
/// only the Tooltip uses it, to render once even when self.mode (seeded from
/// spawn config) already matches the event's mode.
#[allow(clippy::too_many_arguments)]
pub fn decide_mode_update(
    role: Role,
    new_mode: InputMode,
    base_mode: InputMode,
    current_mode: InputMode,
    is_active_clone: bool,
    has_permission: bool,
    hud_is_open: bool,
    tooltip_is_open: bool,
    is_initial: bool,
) -> ModeUpdateDecision {
    match role {
        Role::Hud => {
            // Skip self.mode update on transition to base: the daemon's
            // close_hud pipe will close us shortly, and rendering the
            // base-mode label briefly would flash.
            let new_self_mode = if is_active_clone && new_mode != base_mode {
                Some(new_mode)
            } else {
                None
            };
            // Same flash avoidance applies to the render gate.
            let should_render = !(is_active_clone && new_mode == base_mode);
            ModeUpdateDecision {
                new_self_mode,
                should_render,
                ..ModeUpdateDecision::noop()
            }
        }
        Role::Tooltip => {
            // Active clones react to their own ModeUpdate immediately, to
            // avoid the mode_sync pipe round-trip's perceptible delay.
            // Non-active clones wait for mode_sync (their own ModeUpdate
            // reflects *their* client's mode, not the spawner's).
            let active_change = is_active_clone && current_mode != new_mode;
            let new_self_mode = if active_change { Some(new_mode) } else { None };
            let hidden = is_tooltip_hidden_mode(new_mode, base_mode);
            // Hidden modes (rename / enter_search) skip the resize+render
            // path: the Daemon's close_tooltip pipe will close us shortly.
            let tooltip_active_changed = active_change && !hidden;
            // Initial-render fallback: when self.mode (seeded from spawn
            // config) already matches new_mode, mode_sync will fire with
            // mode_changed=false and won't render — so render once here.
            let initial_render = is_initial
                && (active_change || current_mode == new_mode);
            ModeUpdateDecision {
                new_self_mode,
                tooltip_needs_resize: tooltip_active_changed,
                should_render: tooltip_active_changed || initial_render,
                ..ModeUpdateDecision::noop()
            }
        }
        Role::Daemon => {
            // Daemon always tracks the latest mode (drives the lifecycle).
            let new_self_mode = Some(new_mode);
            if !has_permission {
                // Pre-permission: track mode but skip lifecycle work.
                return ModeUpdateDecision {
                    new_self_mode,
                    should_render: true,
                    ..ModeUpdateDecision::noop()
                };
            }
            let returning_to_base = new_mode == base_mode;
            let close_hud = returning_to_base && hud_is_open;
            let close_tooltip = if returning_to_base {
                tooltip_is_open
            } else {
                is_tooltip_hidden_mode(new_mode, base_mode) && tooltip_is_open
            };
            // daemon_try_spawn handles mode_info_sync for newly spawned
            // instances; we run it on every non-base ModeUpdate.
            let daemon_try_spawn = !returning_to_base;
            ModeUpdateDecision {
                new_self_mode,
                close_hud,
                close_tooltip,
                daemon_try_spawn,
                broadcast_mode_sync: true,
                should_render: true,
                ..ModeUpdateDecision::noop()
            }
        }
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

    // -----------------------------------------------------------------------
    // decide_mode_update
    // -----------------------------------------------------------------------

    /// Helper: build a ModeUpdate decision in tests with the common
    /// argument order spelled out once. Tests that need different inputs
    /// just call this with different positional args (or call
    /// `decide_mode_update` directly).
    fn mu(
        role: Role,
        new: InputMode,
        base: InputMode,
        current: InputMode,
        active: bool,
        perm: bool,
        hud_open: bool,
        tt_open: bool,
        is_initial: bool,
    ) -> ModeUpdateDecision {
        decide_mode_update(role, new, base, current, active, perm, hud_open, tt_open, is_initial)
    }

    // ---- Hud ----

    #[test]
    fn modeupdate_hud_active_clone_updates_self_and_renders() {
        let d = mu(Role::Hud, InputMode::Pane, InputMode::Normal,
                   InputMode::Normal, true, true, false, false, false);
        assert_eq!(d.new_self_mode, Some(InputMode::Pane));
        assert!(d.should_render);
        // HUD never triggers Daemon-only side effects.
        assert!(!d.close_hud);
        assert!(!d.close_tooltip);
        assert!(!d.daemon_try_spawn);
        assert!(!d.broadcast_mode_sync);
    }

    #[test]
    fn modeupdate_hud_active_clone_skips_update_and_render_on_base_transition() {
        // Avoid a "Normal" flash before the daemon's close_hud pipe arrives.
        let d = mu(Role::Hud, InputMode::Normal, InputMode::Normal,
                   InputMode::Pane, true, true, false, false, false);
        assert_eq!(d.new_self_mode, None);
        assert!(!d.should_render);
    }

    #[test]
    fn modeupdate_hud_non_active_clone_skips_update_but_still_renders() {
        // Non-active HUD clones don't react to their own ModeUpdate (they
        // wait for mode_sync), but the render itself isn't suppressed —
        // mode_sync's render arrives later anyway.
        let d = mu(Role::Hud, InputMode::Pane, InputMode::Normal,
                   InputMode::Normal, false, true, false, false, false);
        assert_eq!(d.new_self_mode, None);
        assert!(d.should_render);
    }

    // ---- Tooltip ----

    #[test]
    fn modeupdate_tooltip_active_clone_updates_resizes_and_renders() {
        let d = mu(Role::Tooltip, InputMode::Pane, InputMode::Normal,
                   InputMode::Normal, true, true, false, false, false);
        assert_eq!(d.new_self_mode, Some(InputMode::Pane));
        assert!(d.tooltip_needs_resize);
        assert!(d.should_render);
    }

    #[test]
    fn modeupdate_tooltip_unchanged_mode_is_no_op() {
        // current == new on an active clone → no mutation, no render.
        let d = mu(Role::Tooltip, InputMode::Pane, InputMode::Normal,
                   InputMode::Pane, true, true, false, false, false);
        assert_eq!(d.new_self_mode, None);
        assert!(!d.tooltip_needs_resize);
        assert!(!d.should_render);
    }

    #[test]
    fn modeupdate_tooltip_hidden_mode_updates_but_skips_resize_and_render() {
        // Hidden modes (rename/enter_search): record the mode but skip
        // the resize+render so we don't flash old dimensions.
        let d = mu(Role::Tooltip, InputMode::RenamePane, InputMode::Normal,
                   InputMode::Pane, true, true, false, false, false);
        assert_eq!(d.new_self_mode, Some(InputMode::RenamePane));
        assert!(!d.tooltip_needs_resize);
        assert!(!d.should_render);
    }

    #[test]
    fn modeupdate_tooltip_initial_renders_once_when_mode_already_matches() {
        // Spawn config seeded self.mode = new_mode, so mode_sync will fire
        // mode_changed=false later and not render. We render once here.
        let d = mu(Role::Tooltip, InputMode::Pane, InputMode::Normal,
                   InputMode::Pane, true, true, false, false, true);
        assert_eq!(d.new_self_mode, None);
        assert!(d.should_render);
    }

    #[test]
    fn modeupdate_tooltip_non_active_clone_skips_unless_initial_match() {
        // Non-active clones: skip the immediate-render path. Without
        // is_initial they don't render; with is_initial && self.mode
        // already == new_mode, render once.
        let d = mu(Role::Tooltip, InputMode::Pane, InputMode::Normal,
                   InputMode::Tab, false, true, false, false, false);
        assert!(!d.should_render);

        let d = mu(Role::Tooltip, InputMode::Pane, InputMode::Normal,
                   InputMode::Pane, false, true, false, false, true);
        assert!(d.should_render);
    }

    // ---- Daemon ----

    #[test]
    fn modeupdate_daemon_always_updates_self_mode() {
        // Even pre-permission, the daemon tracks the latest mode so it
        // can act once permission is granted.
        let d = mu(Role::Daemon, InputMode::Pane, InputMode::Normal,
                   InputMode::Normal, true, false, false, false, false);
        assert_eq!(d.new_self_mode, Some(InputMode::Pane));
        // Pre-permission: no lifecycle side effects yet.
        assert!(!d.close_hud);
        assert!(!d.close_tooltip);
        assert!(!d.daemon_try_spawn);
        assert!(!d.broadcast_mode_sync);
    }

    #[test]
    fn modeupdate_daemon_with_permission_in_non_base_mode_spawns_and_broadcasts() {
        let d = mu(Role::Daemon, InputMode::Pane, InputMode::Normal,
                   InputMode::Normal, true, true, false, false, false);
        assert!(d.daemon_try_spawn);
        assert!(d.broadcast_mode_sync);
        assert!(!d.close_hud);
        assert!(!d.close_tooltip);
    }

    #[test]
    fn modeupdate_daemon_returning_to_base_closes_open_panes() {
        let d = mu(Role::Daemon, InputMode::Normal, InputMode::Normal,
                   InputMode::Pane, true, true, /*hud_open=*/true, /*tt_open=*/true, false);
        assert!(d.close_hud);
        assert!(d.close_tooltip);
        assert!(!d.daemon_try_spawn);
        assert!(d.broadcast_mode_sync);
    }

    #[test]
    fn modeupdate_daemon_returning_to_base_skips_close_when_not_open() {
        // close_* is gated on the corresponding *_open flag — no spurious
        // close pipes when there's nothing to close.
        let d = mu(Role::Daemon, InputMode::Normal, InputMode::Normal,
                   InputMode::Pane, true, true, /*hud_open=*/false, /*tt_open=*/false, false);
        assert!(!d.close_hud);
        assert!(!d.close_tooltip);
    }

    #[test]
    fn modeupdate_daemon_hidden_mode_closes_only_open_tooltip() {
        // RenamePane / RenameTab / EnterSearch: keep the HUD up, close
        // the tooltip so its keybinding hints don't obscure the typing
        // target.
        let d = mu(Role::Daemon, InputMode::RenamePane, InputMode::Normal,
                   InputMode::Pane, true, true, /*hud_open=*/true, /*tt_open=*/true, false);
        assert!(!d.close_hud);
        assert!(d.close_tooltip);
        // Still in non-base: spawn path runs.
        assert!(d.daemon_try_spawn);
    }

    #[test]
    fn modeupdate_daemon_hidden_mode_skips_close_when_tooltip_already_closed() {
        // Same as above but tooltip is already closed → no spurious pipe.
        let d = mu(Role::Daemon, InputMode::RenamePane, InputMode::Normal,
                   InputMode::Pane, true, true, true, /*tt_open=*/false, false);
        assert!(!d.close_tooltip);
    }

    #[test]
    fn modeupdate_daemon_always_renders() {
        // Daemon's render() is a no-op (it's the hidden role), but the
        // return value is still true so zellij records the event. Pin
        // that contract.
        let d = mu(Role::Daemon, InputMode::Pane, InputMode::Normal,
                   InputMode::Pane, true, true, false, false, false);
        assert!(d.should_render);
    }
}
