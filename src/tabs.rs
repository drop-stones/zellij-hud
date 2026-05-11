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

#[cfg(test)]
mod tests {
    use super::*;

    fn tab(name: &str, active: bool) -> TabInfo {
        let mut t = TabInfo::default();
        t.name = name.to_string();
        t.active = active;
        t
    }

    #[test]
    fn empty_to_empty_is_unchanged() {
        assert!(!tabs_changed_visibly(&[], &[]));
    }

    #[test]
    fn count_change_is_visible() {
        let one = vec![tab("a", true)];
        let two = vec![tab("a", true), tab("b", false)];
        assert!(tabs_changed_visibly(&[], &one));
        assert!(tabs_changed_visibly(&one, &two));
        assert!(tabs_changed_visibly(&two, &one));
    }

    #[test]
    fn name_change_is_visible() {
        let before = vec![tab("a", true)];
        let after = vec![tab("renamed", true)];
        assert!(tabs_changed_visibly(&before, &after));
    }

    #[test]
    fn active_marker_change_is_visible() {
        // Switching active tab must trigger a re-render of the tab strip.
        let before = vec![tab("a", true), tab("b", false)];
        let after = vec![tab("a", false), tab("b", true)];
        assert!(tabs_changed_visibly(&before, &after));
    }

    #[test]
    fn sync_indicator_change_is_visible() {
        let mut before = tab("a", true);
        let mut after = tab("a", true);
        before.is_sync_panes_active = false;
        after.is_sync_panes_active = true;
        assert!(tabs_changed_visibly(&[before], &[after]));
    }

    #[test]
    fn fullscreen_indicator_change_is_visible() {
        let mut before = tab("a", true);
        let mut after = tab("a", true);
        before.is_fullscreen_active = false;
        after.is_fullscreen_active = true;
        assert!(tabs_changed_visibly(&[before], &[after]));
    }

    #[test]
    fn unchanged_snapshots_are_not_visible() {
        // Same name/active/sync/fullscreen across all entries → false.
        let snap = vec![tab("alpha", true), tab("beta", false)];
        assert!(!tabs_changed_visibly(&snap, &snap.clone()));
    }

    #[test]
    fn changes_to_unwatched_fields_are_ignored() {
        // position is not one of the watched fields; mutate it and verify
        // the function still reports "no visible change". Pin that the
        // predicate intentionally ignores fields the HUD doesn't surface.
        let mut a = tab("a", true);
        let mut b = tab("a", true);
        a.position = 0;
        b.position = 5;
        assert!(!tabs_changed_visibly(&[a], &[b]));
    }
}
