use std::collections::HashSet;

use zellij_tile::prelude::actions::Action;
use zellij_tile::prelude::*;

use crate::action_types::ActionType;

/// A single keybinding entry for tooltip display.
pub struct KeyAction {
    pub key: String,
    pub action_type: ActionType,
    pub description: String,
}

/// A compact "back/exit" key shown at the bottom.
pub struct CommonKey {
    /// Icon representing the key (e.g., 󱊷 for ESC).
    pub icon: &'static str,
    pub description: String,
}

/// Result of keybinding extraction: mode-specific actions + common keys.
pub struct ModeActions {
    pub actions: Vec<KeyAction>,
    pub common: Vec<CommonKey>,
}

/// Text-input modes excluded from shared-key detection
/// (they have very few bindings and would collapse the intersection).
const TEXT_INPUT_MODES: &[InputMode] = &[
    InputMode::RenameTab,
    InputMode::RenamePane,
    InputMode::EnterSearch,
];

/// Collect ALL keybinds for a mode, including shared bindings.
/// `ModeInfo::get_keybinds_for_mode` uses `.find()` internally, which
/// only returns the first entry. If shared bindings are stored as
/// separate entries for the same mode, they would be missed.
fn all_keybinds_for_mode(
    mode_info: &ModeInfo,
    mode: InputMode,
) -> Vec<(KeyWithModifier, Vec<Action>)> {
    let mut result = Vec::new();
    for (m, bindings) in &mode_info.keybinds {
        if *m == mode {
            result.extend(bindings.iter().cloned());
        }
    }
    result
}

/// Extract keybinding hints for a given mode, separating common/back keys.
pub fn get_actions_for_mode(
    mode_info: &ModeInfo,
    mode: InputMode,
) -> ModeActions {
    let keybinds = all_keybinds_for_mode(mode_info, mode);

    // Find (key, ActionType) pairs shared across all non-base, non-text-input modes.
    let base_mode = mode_info.base_mode.unwrap_or(InputMode::Normal);
    let shared = find_shared_bindings(mode_info, base_mode);
    // Navigation back keys (Esc/Enter) detected via "exits via SwitchToMode in
    // every mode" rather than (key, ActionType) pair equality. Catches mode-
    // specific overrides where the binding still exits but the leading action
    // differs (e.g. Scroll's `Esc { ScrollToBottom; SwitchToMode "Locked"; }`).
    let nav_keys = find_navigation_keys(mode_info, base_mode);

    // Collect all keybinds, deduplicating by ActionType (keep shortest key).
    let mut actions: Vec<KeyAction> = Vec::new();

    for (key, key_actions) in &keybinds {
        let first = match key_actions.first() {
            Some(a) => a,
            None => continue,
        };
        let action_type = ActionType::from_action(first);

        // Skip unclassified actions (NoOp and any Action variant we have no
        // typed classification for): they have no description or icon, so
        // they cannot be rendered in the tooltip.
        if matches!(&action_type, ActionType::Other(_)) {
            continue;
        }

        let key_raw = format!("{}", key);

        // Skip navigation back keys — rendered in the bottom common area.
        if nav_keys.contains(&key_raw) {
            continue;
        }
        // Skip if this exact (key, action) pair is shared across all modes
        if shared.contains(&(key_raw.clone(), action_type.clone())) {
            continue;
        }

        let key_str = format_key(&key_raw);

        if let Some(existing) = actions.iter_mut().find(|a| a.action_type == action_type) {
            // Keep shorter key representation
            if key_str.len() < existing.key.len() {
                existing.key = key_str;
            }
        } else {
            let description = action_type.description();
            actions.push(KeyAction {
                key: key_str,
                action_type,
                description,
            });
        }
    }

    // Sort by fixed order: mode switches first, then by semantic group.
    actions.sort_by_key(|a| a.action_type.sort_key());

    // Find iconifiable shared keys for bottom display
    let common = find_common_keys(&shared, &nav_keys, &keybinds);

    ModeActions { actions, common }
}

/// Find (key_string, ActionType) pairs present in ALL non-base, non-text-input modes.
/// These represent shared bindings (e.g., ESC → back to base) that appear identically
/// across modes. Mode-specific overrides using different keys are NOT shared.
fn find_shared_bindings(mode_info: &ModeInfo, base_mode: InputMode) -> HashSet<(String, ActionType)> {
    let modes: HashSet<InputMode> = mode_info
        .keybinds
        .iter()
        .map(|(m, _)| *m)
        .filter(|m| *m != base_mode && !TEXT_INPUT_MODES.contains(m))
        .collect();

    let mut sets: Vec<HashSet<(String, ActionType)>> = Vec::new();

    for m in &modes {
        let keybinds = all_keybinds_for_mode(mode_info, *m);
        let mut pairs = HashSet::new();
        for (key, actions) in &keybinds {
            if let Some(first) = actions.first() {
                pairs.insert((format!("{}", key), ActionType::from_action(first)));
            }
        }
        sets.push(pairs);
    }

    if sets.is_empty() {
        return HashSet::new();
    }

    let mut common = sets[0].clone();
    for s in &sets[1..] {
        common = common.intersection(s).cloned().collect();
    }
    common
}

/// Return the target InputMode of the LAST `SwitchToMode` action in the
/// sequence, or `None` if the binding never exits via SwitchToMode.
/// "Last" matches the runtime semantics: any earlier side effect (e.g.
/// `ScrollToBottom`) runs first, then the mode switch wins.
fn switch_to_mode_target(actions: &[Action]) -> Option<InputMode> {
    actions.iter().rev().find_map(|a| {
        if let Action::SwitchToMode { input_mode } = a {
            Some(*input_mode)
        } else {
            None
        }
    })
}

/// Detect "back/exit" keys (Esc, Enter, ...) that exit every non-base,
/// non-text-input mode via a SwitchToMode action, regardless of any
/// leading side-effect actions. Returns the set of qualifying key strings.
fn find_navigation_keys(mode_info: &ModeInfo, base_mode: InputMode) -> HashSet<String> {
    let modes: Vec<InputMode> = mode_info
        .keybinds
        .iter()
        .map(|(m, _)| *m)
        .filter(|m| *m != base_mode && !TEXT_INPUT_MODES.contains(m))
        .collect();

    if modes.is_empty() {
        return HashSet::new();
    }

    // Candidate keys: those with a tooltip icon, observed in any mode.
    let mut candidates: HashSet<String> = HashSet::new();
    for m in &modes {
        for (key, _) in all_keybinds_for_mode(mode_info, *m) {
            let key_str = format!("{}", key);
            if key_to_icon(&key_str).is_some() {
                candidates.insert(key_str);
            }
        }
    }

    let mut result = HashSet::new();
    for nav_key in candidates {
        let exits_all = modes.iter().all(|m| {
            all_keybinds_for_mode(mode_info, *m)
                .iter()
                .any(|(key, actions)| {
                    format!("{}", key) == nav_key
                        && switch_to_mode_target(actions).is_some()
                })
        });
        if exits_all {
            result.insert(nav_key);
        }
    }
    result
}

/// Find iconifiable keys for compact bottom display.
/// Includes both `(key, ActionType)`-shared keys and navigation back keys.
fn find_common_keys(
    shared: &HashSet<(String, ActionType)>,
    nav_keys: &HashSet<String>,
    keybinds: &[(KeyWithModifier, Vec<Action>)],
) -> Vec<CommonKey> {
    let mut seen_icons = Vec::new();
    let mut common = Vec::new();

    for (key, actions) in keybinds {
        let first = match actions.first() {
            Some(a) => a,
            None => continue,
        };
        let key_str = format!("{}", key);
        let icon = match key_to_icon(&key_str) {
            Some(i) => i,
            None => continue,
        };
        if seen_icons.contains(&icon) {
            continue;
        }

        let action_type = ActionType::from_action(first);
        let from_shared = shared.contains(&(key_str.clone(), action_type.clone()));
        let from_nav = nav_keys.contains(&key_str);
        if !from_shared && !from_nav {
            continue;
        }

        // For nav keys, prefer the SwitchToMode target's name so the user
        // sees where Esc/Enter actually takes them in this mode (e.g. "+scroll"
        // instead of "Scroll bottom" when Esc is `ScrollToBottom; SwitchToMode`).
        let description = if from_nav {
            switch_to_mode_target(actions)
                .map(|m| ActionType::SwitchToMode(m).description())
                .unwrap_or_else(|| action_type.description())
        } else {
            action_type.description()
        };

        seen_icons.push(icon);
        common.push(CommonKey { icon, description });
    }

    common
}

/// Map well-known key names to compact icons.
fn key_to_icon(key: &str) -> Option<&'static str> {
    match key {
        "ESC" | "Esc" => Some("󱊷"),
        "ENTER" | "Enter" => Some("󰌑"),
        _ => None,
    }
}

/// Format a key string for display (Ctrl → C-, Alt → A-).
pub fn format_key(key: &str) -> String {
    if let Some(rest) = key.strip_prefix("Ctrl ") {
        format!("C-{}", rest)
    } else if let Some(rest) = key.strip_prefix("Alt ") {
        format!("A-{}", rest)
    } else {
        key.to_string()
    }
}


