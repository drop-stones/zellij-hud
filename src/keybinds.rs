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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use zellij_tile::prelude::actions::Action;

    fn key(bare: BareKey) -> KeyWithModifier {
        KeyWithModifier {
            bare_key: bare,
            key_modifiers: BTreeSet::new(),
        }
    }

    fn key_mod(bare: BareKey, m: KeyModifier) -> KeyWithModifier {
        let mut mods = BTreeSet::new();
        mods.insert(m);
        KeyWithModifier {
            bare_key: bare,
            key_modifiers: mods,
        }
    }

    fn switch_to(m: InputMode) -> Action {
        Action::SwitchToMode { input_mode: m }
    }

    /// Build a minimal ModeInfo for tests. All fields beyond `mode`,
    /// `base_mode`, and `keybinds` are filled with `Default` / `None`.
    fn mode_info(mode: InputMode, base: InputMode, keybinds: KeybindsVec) -> ModeInfo {
        ModeInfo {
            mode,
            base_mode: Some(base),
            keybinds,
            style: Style::default(),
            capabilities: PluginCapabilities::default(),
            session_name: None,
            editor: None,
            shell: None,
            web_clients_allowed: None,
            web_sharing: None,
            currently_marking_pane_group: None,
            is_web_client: None,
            web_server_ip: None,
            web_server_port: None,
            web_server_capability: None,
        }
    }

    #[test]
    fn format_key_prefixes_modifiers() {
        assert_eq!(format_key("Ctrl g"), "C-g");
        assert_eq!(format_key("Alt h"), "A-h");
        assert_eq!(format_key("p"), "p");
        assert_eq!(format_key("ESC"), "ESC");
    }

    #[test]
    fn key_to_icon_recognizes_esc_and_enter_case_insensitively() {
        assert_eq!(key_to_icon("Esc"), Some("󱊷"));
        assert_eq!(key_to_icon("ESC"), Some("󱊷"));
        assert_eq!(key_to_icon("Enter"), Some("󰌑"));
        assert_eq!(key_to_icon("ENTER"), Some("󰌑"));
        assert_eq!(key_to_icon("p"), None);
        assert_eq!(key_to_icon("Ctrl g"), None);
    }

    #[test]
    fn switch_to_mode_target_returns_last_switch() {
        // Esc { ScrollToBottom; SwitchToMode "Locked"; } — the runtime
        // executes side effects then mode-switches; `last` matches that.
        let actions = vec![
            Action::ScrollToBottom,
            switch_to(InputMode::Locked),
        ];
        assert_eq!(switch_to_mode_target(&actions), Some(InputMode::Locked));
    }

    #[test]
    fn switch_to_mode_target_returns_none_when_no_switch() {
        let actions = vec![Action::ScrollToBottom];
        assert_eq!(switch_to_mode_target(&actions), None);
    }

    #[test]
    fn switch_to_mode_target_picks_last_when_multiple_switches() {
        let actions = vec![
            switch_to(InputMode::Pane),
            Action::ScrollToBottom,
            switch_to(InputMode::Locked),
        ];
        assert_eq!(switch_to_mode_target(&actions), Some(InputMode::Locked));
    }

    #[test]
    fn find_navigation_keys_includes_esc_with_leading_side_effects() {
        // Reproduces the Scroll/Search override that previously broke the
        // (key, ActionType) intersection: even though `Esc` in Scroll mode
        // starts with ScrollToBottom, it still ends in SwitchToMode "Locked"
        // so it must still register as a navigation back key.
        let keybinds: KeybindsVec = vec![
            (InputMode::Locked, vec![]),
            (InputMode::Pane, vec![
                (key(BareKey::Esc), vec![switch_to(InputMode::Locked)]),
            ]),
            (InputMode::Tab, vec![
                (key(BareKey::Esc), vec![switch_to(InputMode::Locked)]),
            ]),
            (InputMode::Scroll, vec![
                (key(BareKey::Esc), vec![
                    Action::ScrollToBottom,
                    switch_to(InputMode::Locked),
                ]),
            ]),
        ];
        let mi = mode_info(InputMode::Pane, InputMode::Locked, keybinds);
        let nav = find_navigation_keys(&mi, InputMode::Locked);
        assert!(nav.contains("ESC"), "expected ESC, got {:?}", nav);
    }

    #[test]
    fn find_navigation_keys_excludes_key_missing_in_some_mode() {
        // Esc exits Pane but Tab does not bind Esc at all → not a nav key.
        let keybinds: KeybindsVec = vec![
            (InputMode::Locked, vec![]),
            (InputMode::Pane, vec![
                (key(BareKey::Esc), vec![switch_to(InputMode::Locked)]),
            ]),
            (InputMode::Tab, vec![
                (key(BareKey::Char('q')), vec![switch_to(InputMode::Locked)]),
            ]),
        ];
        let mi = mode_info(InputMode::Pane, InputMode::Locked, keybinds);
        let nav = find_navigation_keys(&mi, InputMode::Locked);
        assert!(!nav.contains("ESC"));
    }

    #[test]
    fn find_navigation_keys_skips_text_input_modes() {
        // RenameTab/RenamePane/EnterSearch are excluded from the
        // intersection because they have very few bindings; an Esc that
        // exits only the non-text-input modes should still qualify.
        let keybinds: KeybindsVec = vec![
            (InputMode::Locked, vec![]),
            (InputMode::Pane, vec![
                (key(BareKey::Esc), vec![switch_to(InputMode::Locked)]),
            ]),
            (InputMode::Tab, vec![
                (key(BareKey::Esc), vec![switch_to(InputMode::Locked)]),
            ]),
            (InputMode::RenameTab, vec![
                // No Esc binding here, but RenameTab is a text-input mode
                // and is excluded from the all-modes check.
                (key(BareKey::Char('a')), vec![]),
            ]),
        ];
        let mi = mode_info(InputMode::Pane, InputMode::Locked, keybinds);
        let nav = find_navigation_keys(&mi, InputMode::Locked);
        assert!(nav.contains("ESC"));
    }

    #[test]
    fn find_shared_bindings_keeps_only_pairs_in_every_mode() {
        // Ctrl-q -> Quit appears in both modes → shared.
        // n is bound only in Pane → not shared.
        let keybinds: KeybindsVec = vec![
            (InputMode::Locked, vec![]),
            (InputMode::Pane, vec![
                (key_mod(BareKey::Char('q'), KeyModifier::Ctrl), vec![Action::Quit]),
                (key(BareKey::Char('n')), vec![Action::NewPane {
                    direction: None,
                    pane_name: None,
                    start_suppressed: false,
                }]),
            ]),
            (InputMode::Tab, vec![
                (key_mod(BareKey::Char('q'), KeyModifier::Ctrl), vec![Action::Quit]),
            ]),
        ];
        let mi = mode_info(InputMode::Pane, InputMode::Locked, keybinds);
        let shared = find_shared_bindings(&mi, InputMode::Locked);
        assert!(shared.contains(&("Ctrl q".to_string(), ActionType::Quit)));
        assert!(!shared.iter().any(|(k, _)| k == "n"));
    }

    #[test]
    fn get_actions_for_mode_dedupes_by_action_type_and_keeps_shorter_key() {
        // Resize binds both `+` and `=` to ResizeIncreaseAll. The shorter
        // representation should be kept.  A second non-base mode is needed
        // so find_shared_bindings does not classify these as cross-mode
        // common bindings.
        let resize_bindings = vec![
            (key(BareKey::Char('+')), vec![Action::Resize {
                resize: Resize::Increase, direction: None,
            }]),
            (key(BareKey::Char('=')), vec![Action::Resize {
                resize: Resize::Increase, direction: None,
            }]),
        ];
        let keybinds: KeybindsVec = vec![
            (InputMode::Locked, vec![]),
            (InputMode::Resize, resize_bindings),
            (InputMode::Pane, vec![
                (key(BareKey::Char('h')), vec![Action::MoveFocus { direction: Direction::Left }]),
            ]),
        ];
        let mi = mode_info(InputMode::Resize, InputMode::Locked, keybinds);
        let result = get_actions_for_mode(&mi, InputMode::Resize);
        let increase = result.actions.iter()
            .find(|a| a.action_type == ActionType::ResizeIncreaseAll)
            .expect("ResizeIncreaseAll should appear once");
        assert_eq!(increase.key.len(), 1);
        assert_eq!(
            result.actions.iter()
                .filter(|a| a.action_type == ActionType::ResizeIncreaseAll)
                .count(),
            1,
            "ResizeIncreaseAll should be deduplicated",
        );
    }

    #[test]
    fn get_actions_for_mode_routes_esc_to_common_with_target_label() {
        // Scroll has Esc { ScrollToBottom; SwitchToMode "Locked"; }
        // Expectation: Esc shows up as a "common" navigation key with the
        // SwitchToMode target's description (not "Scroll bottom").
        let keybinds: KeybindsVec = vec![
            (InputMode::Locked, vec![]),
            (InputMode::Pane, vec![
                (key(BareKey::Esc), vec![switch_to(InputMode::Locked)]),
            ]),
            (InputMode::Scroll, vec![
                (key(BareKey::Esc), vec![
                    Action::ScrollToBottom,
                    switch_to(InputMode::Locked),
                ]),
            ]),
        ];
        let mi = mode_info(InputMode::Scroll, InputMode::Locked, keybinds);
        let result = get_actions_for_mode(&mi, InputMode::Scroll);

        let esc_common = result.common.iter()
            .find(|c| c.icon == "󱊷")
            .expect("Esc icon should appear in common");
        assert_eq!(
            esc_common.description,
            ActionType::SwitchToMode(InputMode::Locked).description(),
        );

        // Esc must NOT appear in the per-mode action list (it is rendered in
        // the bottom common area instead).
        assert!(
            !result.actions.iter().any(|a| a.key.contains("ESC")),
            "Esc should be excluded from per-mode actions: {:?}",
            result.actions.iter().map(|a| &a.key).collect::<Vec<_>>(),
        );
    }
}
