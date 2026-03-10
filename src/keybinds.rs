use std::collections::HashSet;

use zellij_tile::prelude::actions::{Action, SearchDirection, SearchOption};
use zellij_tile::prelude::*;

use crate::action_types::ActionType;

/// A single keybinding entry for tooltip display.
pub(crate) struct KeyAction {
    pub(crate) key: String,
    pub(crate) action_type: ActionType,
    pub(crate) description: String,
}

/// A compact "back/exit" key shown at the bottom.
pub(crate) struct CommonKey {
    /// Icon representing the key (e.g., 󱊷 for ESC).
    pub(crate) icon: &'static str,
    pub(crate) description: String,
}

/// Result of keybinding extraction: mode-specific actions + common keys.
pub(crate) struct ModeActions {
    pub(crate) actions: Vec<KeyAction>,
    pub(crate) common: Vec<CommonKey>,
}

/// Modes where the tooltip is shown (for common key detection).
const TOOLTIP_MODES: &[InputMode] = &[
    InputMode::Normal,
    InputMode::Pane,
    InputMode::Tab,
    InputMode::Resize,
    InputMode::Move,
    InputMode::Scroll,
    InputMode::Search,
    InputMode::Session,
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
pub(crate) fn get_actions_for_mode(
    mode_info: &ModeInfo,
    mode: InputMode,
) -> ModeActions {
    let predicates = mode_predicates(mode);
    let all_actions = find_actions(mode_info, mode, &predicates);

    // Find ActionTypes common to ALL tooltip modes (e.g., Quit, SwitchToMode(Locked))
    let common_types = find_common_action_types(mode_info);

    // Filter out common actions from main list
    let actions: Vec<KeyAction> = all_actions
        .into_iter()
        .filter(|a| !common_types.contains(&a.action_type))
        .collect();

    // Find iconifiable common keys for bottom display
    let common = find_common_keys(mode_info, mode, &common_types);

    ModeActions { actions, common }
}

/// Find ActionTypes that appear in ALL tooltip-visible modes.
fn find_common_action_types(mode_info: &ModeInfo) -> HashSet<ActionType> {
    let mut sets: Vec<HashSet<ActionType>> = Vec::new();

    for &m in TOOLTIP_MODES {
        let keybinds = all_keybinds_for_mode(mode_info, m);
        let mut mode_types = HashSet::new();
        for (_key, actions) in &keybinds {
            if let Some(first) = actions.first() {
                mode_types.insert(ActionType::from_action(first));
            }
        }
        sets.push(mode_types);
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

/// Find keys for common actions (shared across all tooltip modes)
/// and convert iconifiable ones to compact display form.
fn find_common_keys(
    mode_info: &ModeInfo,
    mode: InputMode,
    common_types: &HashSet<ActionType>,
) -> Vec<CommonKey> {
    let keybinds = all_keybinds_for_mode(mode_info, mode);

    let mut seen_icons = Vec::new();
    let mut common = Vec::new();

    for (key, actions) in &keybinds {
        if let Some(first) = actions.first() {
            let action_type = ActionType::from_action(first);
            if common_types.contains(&action_type) {
                let key_str = format!("{}", key);
                if let Some(icon) = key_to_icon(&key_str) {
                    if !seen_icons.contains(&icon) {
                        seen_icons.push(icon);
                        common.push(CommonKey {
                            icon,
                            description: action_type.description(),
                        });
                    }
                }
            }
        }
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
pub(crate) fn format_key(key: &str) -> String {
    if let Some(rest) = key.strip_prefix("Ctrl ") {
        format!("C-{}", rest)
    } else if let Some(rest) = key.strip_prefix("Alt ") {
        format!("A-{}", rest)
    } else {
        key.to_string()
    }
}

/// For each predicate, find the shortest matching key.
/// One key per line, no grouping.
fn find_actions(
    mode_info: &ModeInfo,
    mode: InputMode,
    predicates: &[fn(&Action) -> bool],
) -> Vec<KeyAction> {
    let keybinds = all_keybinds_for_mode(mode_info, mode);
    let mut result = Vec::new();

    for predicate in predicates {
        let mut best_key: Option<String> = None;
        let mut matched_action: Option<&Action> = None;

        for (key, actions) in &keybinds {
            if let Some(first) = actions.first() {
                if predicate(first) {
                    let key_str = format!("{}", key);
                    if best_key
                        .as_ref()
                        .map_or(true, |bk| key_str.len() < bk.len())
                    {
                        best_key = Some(key_str);
                        matched_action = Some(first);
                    }
                }
            }
        }

        if let (Some(key), Some(action)) = (best_key, matched_action) {
            result.push(KeyAction {
                key: format_key(&key),
                action_type: ActionType::from_action(action),
                description: description_for_action(action),
            });
        }
    }

    result
}

/// Generate a context-specific description from the raw Action.
fn description_for_action(action: &Action) -> String {
    match action {
        Action::MoveFocus(Direction::Left) => "Move left".into(),
        Action::MoveFocus(Direction::Down) => "Move down".into(),
        Action::MoveFocus(Direction::Up) => "Move up".into(),
        Action::MoveFocus(Direction::Right) => "Move right".into(),
        Action::MovePane(Some(Direction::Left)) => "Move pane left".into(),
        Action::MovePane(Some(Direction::Down)) => "Move pane down".into(),
        Action::MovePane(Some(Direction::Up)) => "Move pane up".into(),
        Action::MovePane(Some(Direction::Right)) => "Move pane right".into(),
        Action::Resize(Resize::Increase, None) => "Increase size".into(),
        Action::Resize(Resize::Decrease, None) => "Decrease size".into(),
        Action::Resize(Resize::Increase, Some(Direction::Left)) => "Grow left".into(),
        Action::Resize(Resize::Increase, Some(Direction::Down)) => "Grow down".into(),
        Action::Resize(Resize::Increase, Some(Direction::Up)) => "Grow up".into(),
        Action::Resize(Resize::Increase, Some(Direction::Right)) => "Grow right".into(),
        Action::Resize(Resize::Decrease, Some(Direction::Left)) => "Shrink left".into(),
        Action::Resize(Resize::Decrease, Some(Direction::Down)) => "Shrink down".into(),
        Action::Resize(Resize::Decrease, Some(Direction::Up)) => "Shrink up".into(),
        Action::Resize(Resize::Decrease, Some(Direction::Right)) => "Shrink right".into(),
        Action::GoToPreviousTab => "Previous tab".into(),
        Action::GoToNextTab => "Next tab".into(),
        Action::ScrollUp => "Scroll up".into(),
        Action::ScrollDown => "Scroll down".into(),
        Action::PageScrollUp => "Page up".into(),
        Action::PageScrollDown => "Page down".into(),
        Action::HalfPageScrollUp => "Half page up".into(),
        Action::HalfPageScrollDown => "Half page down".into(),
        Action::Search(SearchDirection::Down) => "Next match".into(),
        Action::Search(SearchDirection::Up) => "Previous match".into(),
        Action::BreakPaneLeft => "Break pane left".into(),
        Action::BreakPaneRight => "Break pane right".into(),
        other => ActionType::from_action(other).description(),
    }
}

/// Returns ordered predicates for each mode.
fn mode_predicates(mode: InputMode) -> Vec<fn(&Action) -> bool> {
    match mode {
        InputMode::Locked => vec![
            |a| matches!(a, Action::SwitchToMode(InputMode::Normal)),
        ],
        InputMode::Normal => vec![
            |a| matches!(a, Action::SwitchToMode(InputMode::Locked)),
            |a| matches!(a, Action::SwitchToMode(InputMode::Pane)),
            |a| matches!(a, Action::SwitchToMode(InputMode::Tab)),
            |a| matches!(a, Action::SwitchToMode(InputMode::Resize)),
            |a| matches!(a, Action::SwitchToMode(InputMode::Move)),
            |a| matches!(a, Action::SwitchToMode(InputMode::Scroll)),
            |a| matches!(a, Action::SwitchToMode(InputMode::Session)),
            |a| matches!(a, Action::Quit),
        ],
        InputMode::Pane => vec![
            |a| matches!(a, Action::MoveFocus(Direction::Left)),
            |a| matches!(a, Action::MoveFocus(Direction::Down)),
            |a| matches!(a, Action::MoveFocus(Direction::Up)),
            |a| matches!(a, Action::MoveFocus(Direction::Right)),
            |a| matches!(a, Action::NewPane(None, _, _)),
            |a| matches!(a, Action::NewPane(Some(Direction::Down), _, _)),
            |a| matches!(a, Action::NewPane(Some(Direction::Right), _, _)),
            |a| matches!(a, Action::CloseFocus),
            |a| matches!(a, Action::SwitchToMode(InputMode::RenamePane)),
            |a| matches!(a, Action::ToggleFocusFullscreen),
            |a| matches!(a, Action::ToggleFloatingPanes),
            |a| matches!(a, Action::TogglePaneEmbedOrFloating),
            |a| matches!(a, Action::NewStackedPane(..)),
        ],
        InputMode::Tab => vec![
            |a| matches!(a, Action::GoToPreviousTab),
            |a| matches!(a, Action::GoToNextTab),
            |a| matches!(a, Action::NewTab(..)),
            |a| matches!(a, Action::CloseTab),
            |a| matches!(a, Action::SwitchToMode(InputMode::RenameTab)),
            |a| matches!(a, Action::ToggleActiveSyncTab),
            |a| matches!(a, Action::BreakPane),
            |a| matches!(a, Action::BreakPaneLeft),
            |a| matches!(a, Action::ToggleTab),
        ],
        InputMode::Resize => vec![
            |a| matches!(a, Action::Resize(Resize::Increase, None)),
            |a| matches!(a, Action::Resize(Resize::Decrease, None)),
            |a| matches!(a, Action::Resize(Resize::Increase, Some(Direction::Left))),
            |a| matches!(a, Action::Resize(Resize::Increase, Some(Direction::Down))),
            |a| matches!(a, Action::Resize(Resize::Increase, Some(Direction::Up))),
            |a| matches!(a, Action::Resize(Resize::Increase, Some(Direction::Right))),
            |a| matches!(a, Action::Resize(Resize::Decrease, Some(Direction::Left))),
            |a| matches!(a, Action::Resize(Resize::Decrease, Some(Direction::Down))),
            |a| matches!(a, Action::Resize(Resize::Decrease, Some(Direction::Up))),
            |a| matches!(a, Action::Resize(Resize::Decrease, Some(Direction::Right))),
        ],
        InputMode::Move => vec![
            |a| matches!(a, Action::MovePane(Some(Direction::Left))),
            |a| matches!(a, Action::MovePane(Some(Direction::Down))),
            |a| matches!(a, Action::MovePane(Some(Direction::Up))),
            |a| matches!(a, Action::MovePane(Some(Direction::Right))),
        ],
        InputMode::Scroll => vec![
            |a| matches!(a, Action::ScrollDown),
            |a| matches!(a, Action::ScrollUp),
            |a| matches!(a, Action::HalfPageScrollDown),
            |a| matches!(a, Action::HalfPageScrollUp),
            |a| matches!(a, Action::PageScrollDown),
            |a| matches!(a, Action::PageScrollUp),
            |a| matches!(a, Action::SwitchToMode(InputMode::EnterSearch)),
            |a| matches!(a, Action::EditScrollback),
        ],
        InputMode::Search => vec![
            |a| matches!(a, Action::SwitchToMode(InputMode::EnterSearch)),
            |a| matches!(a, Action::ScrollDown),
            |a| matches!(a, Action::ScrollUp),
            |a| matches!(a, Action::PageScrollDown),
            |a| matches!(a, Action::PageScrollUp),
            |a| matches!(a, Action::HalfPageScrollDown),
            |a| matches!(a, Action::HalfPageScrollUp),
            |a| matches!(a, Action::Search(SearchDirection::Down)),
            |a| matches!(a, Action::Search(SearchDirection::Up)),
            |a| matches!(a, Action::SearchToggleOption(SearchOption::CaseSensitivity)),
            |a| matches!(a, Action::SearchToggleOption(SearchOption::Wrap)),
            |a| matches!(a, Action::SearchToggleOption(SearchOption::WholeWord)),
        ],
        InputMode::Session => vec![
            |a| matches!(a, Action::Detach),
            |a| a.launches_plugin("session-manager"),
            |a| a.launches_plugin("plugin-manager"),
            |a| a.launches_plugin("configuration"),
        ],
        InputMode::EnterSearch
        | InputMode::RenameTab
        | InputMode::RenamePane
        | InputMode::Prompt
        | InputMode::Tmux => vec![],
    }
}
