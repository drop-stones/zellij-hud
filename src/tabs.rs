//! Pure predicates over `TabInfo` slices.

use zellij_tile::prelude::TabInfo;

/// Whether two tab snapshots differ in any field the HUD or tooltip cares
/// about for re-render scheduling.
///
/// The HUD shows: tab name, active marker, sync indicator, fullscreen
/// indicator. Anything else (focused pane id, internal layout) does not
/// affect what the user sees and would only cause render churn.
pub fn tabs_changed_visibly(old: &[TabInfo], new: &[TabInfo]) -> bool {
    if old.len() != new.len() {
        return true;
    }
    old.iter().zip(new.iter()).any(|(a, b)| {
        a.active != b.active
            || a.name != b.name
            || a.is_sync_panes_active != b.is_sync_panes_active
            || a.is_fullscreen_active != b.is_fullscreen_active
    })
}
