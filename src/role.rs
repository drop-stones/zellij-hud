//! The plugin's role marker.

/// Plugin role within the zellij-hud system.
///
/// One WASM binary, three roles distinguished at load time by config flags:
/// the Daemon stays hidden and orchestrates the lifecycle, the HUD renders
/// the floating status bar, the Tooltip renders the which-key keybinding
/// hints.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Role {
    /// Background daemon that spawns HUD and tooltip panes.
    #[default]
    Daemon,
    /// Floating status bar at the bottom.
    Hud,
    /// Floating which-key tooltip at the bottom-right.
    Tooltip,
}
