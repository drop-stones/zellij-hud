use zellij_tile::prelude::actions::Action;
use zellij_tile::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActionType {
    // Mode switches
    SwitchToMode(InputMode),

    // Navigation — focus movement
    MoveFocusLeft,
    MoveFocusDown,
    MoveFocusUp,
    MoveFocusRight,

    // Navigation — tabs
    GoToPreviousTab,
    GoToNextTab,
    ToggleTab,

    // Navigation — scroll
    ScrollDown,
    ScrollUp,
    HalfPageScrollDown,
    HalfPageScrollUp,
    PageScrollDown,
    PageScrollUp,

    // Navigation — search
    SearchDown,
    SearchUp,
    SearchToggleCaseSensitivity,
    SearchToggleWrap,
    SearchToggleWholeWord,

    // Create
    NewPaneWithoutDirection,
    NewPaneDown,
    NewPaneRight,
    NewStackedPane,
    NewTab,

    // Modify / move — pane movement
    MovePaneLeft,
    MovePaneDown,
    MovePaneUp,
    MovePaneRight,

    // Modify / move — resize
    ResizeIncreaseAll,
    ResizeDecreaseAll,
    ResizeIncreaseLeft,
    ResizeIncreaseDown,
    ResizeIncreaseUp,
    ResizeIncreaseRight,
    ResizeDecreaseLeft,
    ResizeDecreaseDown,
    ResizeDecreaseUp,
    ResizeDecreaseRight,

    // Modify / move — tab movement
    MoveTabLeft,
    MoveTabRight,

    // Modify / move — break
    BreakPane,
    BreakPaneLeft,
    BreakPaneRight,

    // Toggle
    ToggleFocusFullscreen,
    ToggleFloatingPanes,
    TogglePaneEmbedOrFloating,
    ToggleActiveSyncTab,

    // Close / exit
    CloseFocus,
    CloseTab,
    Detach,
    Quit,

    // Edit
    EditScrollback,

    // Plugins
    SessionManager,
    PluginManager,
    Configuration,
    LaunchPlugin(String),

    // Unknown
    Other(String),
}

impl ActionType {
    /// Fixed sort order for tooltip display.
    pub fn sort_key(&self) -> u16 {
        match self {
            // Mode switches (0–19)
            ActionType::SwitchToMode(m) => match m {
                InputMode::Locked => 0,
                InputMode::Normal => 1,
                InputMode::Pane => 2,
                InputMode::Tab => 3,
                InputMode::Resize => 4,
                InputMode::Move => 5,
                InputMode::Scroll => 6,
                InputMode::Search => 7,
                InputMode::EnterSearch => 8,
                InputMode::Session => 9,
                InputMode::RenameTab => 10,
                InputMode::RenamePane => 11,
                InputMode::Tmux => 12,
                InputMode::Prompt => 13,
            },

            // Navigation (20–39)
            ActionType::MoveFocusLeft => 20,
            ActionType::MoveFocusDown => 21,
            ActionType::MoveFocusUp => 22,
            ActionType::MoveFocusRight => 23,
            ActionType::GoToPreviousTab => 24,
            ActionType::GoToNextTab => 25,
            ActionType::ToggleTab => 26,
            ActionType::ScrollDown => 27,
            ActionType::ScrollUp => 28,
            ActionType::HalfPageScrollDown => 29,
            ActionType::HalfPageScrollUp => 30,
            ActionType::PageScrollDown => 31,
            ActionType::PageScrollUp => 32,
            ActionType::SearchDown => 33,
            ActionType::SearchUp => 34,
            ActionType::SearchToggleCaseSensitivity => 35,
            ActionType::SearchToggleWrap => 36,
            ActionType::SearchToggleWholeWord => 37,

            // Create (40–49)
            ActionType::NewPaneWithoutDirection => 40,
            ActionType::NewPaneDown => 41,
            ActionType::NewPaneRight => 42,
            ActionType::NewStackedPane => 43,
            ActionType::NewTab => 44,

            // Modify / move (50–69)
            ActionType::MovePaneLeft => 50,
            ActionType::MovePaneDown => 51,
            ActionType::MovePaneUp => 52,
            ActionType::MovePaneRight => 53,
            ActionType::ResizeIncreaseAll => 54,
            ActionType::ResizeDecreaseAll => 55,
            ActionType::ResizeIncreaseLeft => 56,
            ActionType::ResizeIncreaseDown => 57,
            ActionType::ResizeIncreaseUp => 58,
            ActionType::ResizeIncreaseRight => 59,
            ActionType::ResizeDecreaseLeft => 60,
            ActionType::ResizeDecreaseDown => 61,
            ActionType::ResizeDecreaseUp => 62,
            ActionType::ResizeDecreaseRight => 63,
            ActionType::MoveTabLeft => 64,
            ActionType::MoveTabRight => 65,
            ActionType::BreakPane => 66,
            ActionType::BreakPaneLeft => 67,
            ActionType::BreakPaneRight => 68,

            // Toggle (70–79)
            ActionType::ToggleFocusFullscreen => 70,
            ActionType::ToggleFloatingPanes => 71,
            ActionType::TogglePaneEmbedOrFloating => 72,
            ActionType::ToggleActiveSyncTab => 73,

            // Close / exit (80–89)
            ActionType::CloseFocus => 80,
            ActionType::CloseTab => 81,
            ActionType::Detach => 82,
            ActionType::Quit => 83,

            // Edit (90–94)
            ActionType::EditScrollback => 90,

            // Plugins (95–99)
            ActionType::SessionManager => 95,
            ActionType::PluginManager => 96,
            ActionType::Configuration => 97,
            ActionType::LaunchPlugin(_) => 98,

            // Unknown (100+)
            ActionType::Other(_) => 100,
        }
    }

    pub fn description(&self) -> String {
        match self {
            ActionType::SwitchToMode(m) if *m == InputMode::RenamePane => "+rename-pane".into(),
            ActionType::SwitchToMode(m) if *m == InputMode::RenameTab => "+rename-tab".into(),
            ActionType::SwitchToMode(m) if *m == InputMode::EnterSearch => "+search".into(),
            ActionType::SwitchToMode(m) if *m == InputMode::Locked => "+locked".into(),
            ActionType::SwitchToMode(m) if *m == InputMode::Normal => "+normal".into(),
            ActionType::SwitchToMode(m) => format!("+{}", format!("{:?}", m).to_lowercase()),

            ActionType::MoveFocusLeft => "Move left".into(),
            ActionType::MoveFocusDown => "Move down".into(),
            ActionType::MoveFocusUp => "Move up".into(),
            ActionType::MoveFocusRight => "Move right".into(),
            ActionType::GoToPreviousTab => "Previous tab".into(),
            ActionType::GoToNextTab => "Next tab".into(),
            ActionType::ToggleTab => "Circle tab".into(),
            ActionType::ScrollDown => "Scroll down".into(),
            ActionType::ScrollUp => "Scroll up".into(),
            ActionType::HalfPageScrollDown => "Half page down".into(),
            ActionType::HalfPageScrollUp => "Half page up".into(),
            ActionType::PageScrollDown => "Page down".into(),
            ActionType::PageScrollUp => "Page up".into(),
            ActionType::SearchDown => "Next match".into(),
            ActionType::SearchUp => "Previous match".into(),
            ActionType::SearchToggleCaseSensitivity => "Case sensitivity".into(),
            ActionType::SearchToggleWrap => "Wrap".into(),
            ActionType::SearchToggleWholeWord => "Whole word".into(),

            ActionType::NewPaneWithoutDirection => "New pane".into(),
            ActionType::NewPaneDown => "Horizontal split".into(),
            ActionType::NewPaneRight => "Vertical split".into(),
            ActionType::NewStackedPane => "Stacked pane".into(),
            ActionType::NewTab => "New tab".into(),

            ActionType::MoveTabLeft => "Move tab left".into(),
            ActionType::MoveTabRight => "Move tab right".into(),
            ActionType::MovePaneLeft => "Move pane left".into(),
            ActionType::MovePaneDown => "Move pane down".into(),
            ActionType::MovePaneUp => "Move pane up".into(),
            ActionType::MovePaneRight => "Move pane right".into(),
            ActionType::ResizeIncreaseAll => "Increase size".into(),
            ActionType::ResizeDecreaseAll => "Decrease size".into(),
            ActionType::ResizeIncreaseLeft => "Grow left".into(),
            ActionType::ResizeIncreaseDown => "Grow down".into(),
            ActionType::ResizeIncreaseUp => "Grow up".into(),
            ActionType::ResizeIncreaseRight => "Grow right".into(),
            ActionType::ResizeDecreaseLeft => "Shrink left".into(),
            ActionType::ResizeDecreaseDown => "Shrink down".into(),
            ActionType::ResizeDecreaseUp => "Shrink up".into(),
            ActionType::ResizeDecreaseRight => "Shrink right".into(),
            ActionType::BreakPane => "Break pane".into(),
            ActionType::BreakPaneLeft => "Break pane left".into(),
            ActionType::BreakPaneRight => "Break pane right".into(),

            ActionType::ToggleFocusFullscreen => "Fullscreen".into(),
            ActionType::ToggleFloatingPanes => "Toggle floating".into(),
            ActionType::TogglePaneEmbedOrFloating => "Float/embed".into(),
            ActionType::ToggleActiveSyncTab => "Sync tab".into(),

            ActionType::CloseFocus => "Close pane".into(),
            ActionType::CloseTab => "Close tab".into(),
            ActionType::Detach => "Detach".into(),
            ActionType::Quit => "Quit".into(),

            ActionType::EditScrollback => "Edit scrollback".into(),

            ActionType::SessionManager => "Session manager".into(),
            ActionType::PluginManager => "Plugin manager".into(),
            ActionType::Configuration => "Configuration".into(),
            ActionType::LaunchPlugin(name) => plugin_display_name(name),
            ActionType::Other(_) => "Other".into(),
        }
    }

    /// Whether this action switches to another input mode.
    pub fn is_mode_switch(&self) -> bool {
        matches!(self, ActionType::SwitchToMode(_))
    }

    /// Icon for the action type.
    pub fn icon(&self) -> &str {
        match self {
            ActionType::SwitchToMode(m) => match m {
                InputMode::Normal => "󰍀",
                InputMode::Locked => "󰌾",
                InputMode::Pane => "󰘖",
                InputMode::Tab => "󰓩",
                InputMode::Resize => "󰩨",
                InputMode::Move => "󰆾",
                InputMode::Scroll => "󰠶",
                InputMode::Session => "󱂬",
                InputMode::Search | InputMode::EnterSearch => "󰍉",
                InputMode::RenameTab | InputMode::RenamePane => "󰏫",
                InputMode::Tmux => "󰰣",
                InputMode::Prompt => "󰘥",
            },
            ActionType::MoveFocusLeft
            | ActionType::MoveFocusDown
            | ActionType::MoveFocusUp
            | ActionType::MoveFocusRight => "󰁌",
            ActionType::MovePaneLeft
            | ActionType::MovePaneDown
            | ActionType::MovePaneUp
            | ActionType::MovePaneRight => "󰁌",
            ActionType::ResizeIncreaseAll
            | ActionType::ResizeDecreaseAll
            | ActionType::ResizeIncreaseLeft
            | ActionType::ResizeIncreaseDown
            | ActionType::ResizeIncreaseUp
            | ActionType::ResizeIncreaseRight
            | ActionType::ResizeDecreaseLeft
            | ActionType::ResizeDecreaseDown
            | ActionType::ResizeDecreaseUp
            | ActionType::ResizeDecreaseRight => "󰩨",
            ActionType::NewPaneDown
            | ActionType::NewPaneRight
            | ActionType::NewPaneWithoutDirection
            | ActionType::NewStackedPane
            | ActionType::NewTab => "󰐕",
            ActionType::CloseFocus | ActionType::CloseTab => "󰅖",
            ActionType::ToggleFocusFullscreen => "󰊓",
            ActionType::ToggleFloatingPanes | ActionType::TogglePaneEmbedOrFloating => "󰉈",
            ActionType::GoToPreviousTab | ActionType::GoToNextTab => "󰓩",
            ActionType::MoveTabLeft | ActionType::MoveTabRight => "󰓩",
            ActionType::BreakPane | ActionType::BreakPaneLeft | ActionType::BreakPaneRight => "󰀞",
            ActionType::ToggleActiveSyncTab => "󰓦",
            ActionType::ToggleTab => "󰑍",
            ActionType::ScrollDown
            | ActionType::ScrollUp
            | ActionType::PageScrollDown
            | ActionType::PageScrollUp
            | ActionType::HalfPageScrollDown
            | ActionType::HalfPageScrollUp => "󰠶",
            ActionType::SearchDown
            | ActionType::SearchUp
            | ActionType::SearchToggleCaseSensitivity
            | ActionType::SearchToggleWrap
            | ActionType::SearchToggleWholeWord => "󰍉",
            ActionType::EditScrollback => "󰏫",
            ActionType::SessionManager => "󱂬",
            ActionType::Configuration => "󰒓",
            ActionType::PluginManager => "󰏗",
            ActionType::LaunchPlugin(_) => "󰘳",
            ActionType::Detach => "󰗼",
            ActionType::Quit => "󰈆",
            ActionType::Other(_) => "󰘳",
        }
    }

    /// Color for the icon, derived from the theme palette.
    pub fn icon_color<'a>(
        &self,
        colors: &'a crate::config::IconColors,
    ) -> &'a crate::config::Color {
        match self {
            ActionType::SwitchToMode(_) => &colors.mode_switch,
            ActionType::MoveFocusLeft
            | ActionType::MoveFocusDown
            | ActionType::MoveFocusUp
            | ActionType::MoveFocusRight => &colors.navigation,
            ActionType::MovePaneLeft
            | ActionType::MovePaneDown
            | ActionType::MovePaneUp
            | ActionType::MovePaneRight => &colors.navigation,
            ActionType::GoToPreviousTab
            | ActionType::GoToNextTab
            | ActionType::MoveTabLeft
            | ActionType::MoveTabRight
            | ActionType::ToggleTab => &colors.navigation,
            ActionType::ResizeIncreaseAll
            | ActionType::ResizeDecreaseAll
            | ActionType::ResizeIncreaseLeft
            | ActionType::ResizeIncreaseDown
            | ActionType::ResizeIncreaseUp
            | ActionType::ResizeIncreaseRight
            | ActionType::ResizeDecreaseLeft
            | ActionType::ResizeDecreaseDown
            | ActionType::ResizeDecreaseUp
            | ActionType::ResizeDecreaseRight => &colors.resize,
            ActionType::NewPaneDown
            | ActionType::NewPaneRight
            | ActionType::NewPaneWithoutDirection
            | ActionType::NewStackedPane
            | ActionType::NewTab => &colors.create,
            ActionType::CloseFocus | ActionType::CloseTab | ActionType::Quit => &colors.close,
            ActionType::Detach => &colors.close,
            ActionType::ToggleFocusFullscreen
            | ActionType::ToggleFloatingPanes
            | ActionType::TogglePaneEmbedOrFloating
            | ActionType::ToggleActiveSyncTab => &colors.toggle,
            ActionType::BreakPane
            | ActionType::BreakPaneLeft
            | ActionType::BreakPaneRight => &colors.resize,
            ActionType::ScrollDown
            | ActionType::ScrollUp
            | ActionType::PageScrollDown
            | ActionType::PageScrollUp
            | ActionType::HalfPageScrollDown
            | ActionType::HalfPageScrollUp => &colors.navigation,
            ActionType::SearchDown
            | ActionType::SearchUp
            | ActionType::SearchToggleCaseSensitivity
            | ActionType::SearchToggleWrap
            | ActionType::SearchToggleWholeWord => &colors.search,
            ActionType::EditScrollback => &colors.search,
            ActionType::SessionManager
            | ActionType::Configuration
            | ActionType::PluginManager => &colors.create,
            ActionType::LaunchPlugin(_) => &colors.plugin,
            ActionType::Other(_) => &colors.dim,
        }
    }

    pub fn from_action(action: &Action) -> Self {
        use actions::SearchDirection;
        use actions::SearchOption;

        match action {
            Action::MoveFocus { direction: Direction::Left } => ActionType::MoveFocusLeft,
            Action::MoveFocus { direction: Direction::Down } => ActionType::MoveFocusDown,
            Action::MoveFocus { direction: Direction::Up } => ActionType::MoveFocusUp,
            Action::MoveFocus { direction: Direction::Right } => ActionType::MoveFocusRight,

            Action::MovePane { direction: Some(Direction::Left) } => ActionType::MovePaneLeft,
            Action::MovePane { direction: Some(Direction::Down) } => ActionType::MovePaneDown,
            Action::MovePane { direction: Some(Direction::Up) } => ActionType::MovePaneUp,
            Action::MovePane { direction: Some(Direction::Right) } => ActionType::MovePaneRight,

            Action::Resize { resize: Resize::Increase, direction: None } => ActionType::ResizeIncreaseAll,
            Action::Resize { resize: Resize::Decrease, direction: None } => ActionType::ResizeDecreaseAll,
            Action::Resize { resize: Resize::Increase, direction: Some(Direction::Left) } => ActionType::ResizeIncreaseLeft,
            Action::Resize { resize: Resize::Increase, direction: Some(Direction::Down) } => ActionType::ResizeIncreaseDown,
            Action::Resize { resize: Resize::Increase, direction: Some(Direction::Up) } => ActionType::ResizeIncreaseUp,
            Action::Resize { resize: Resize::Increase, direction: Some(Direction::Right) } => ActionType::ResizeIncreaseRight,
            Action::Resize { resize: Resize::Decrease, direction: Some(Direction::Left) } => ActionType::ResizeDecreaseLeft,
            Action::Resize { resize: Resize::Decrease, direction: Some(Direction::Down) } => ActionType::ResizeDecreaseDown,
            Action::Resize { resize: Resize::Decrease, direction: Some(Direction::Up) } => ActionType::ResizeDecreaseUp,
            Action::Resize { resize: Resize::Decrease, direction: Some(Direction::Right) } => ActionType::ResizeDecreaseRight,

            Action::Search { direction: SearchDirection::Down } => ActionType::SearchDown,
            Action::Search { direction: SearchDirection::Up } => ActionType::SearchUp,
            Action::SearchToggleOption { option: SearchOption::CaseSensitivity } => ActionType::SearchToggleCaseSensitivity,
            Action::SearchToggleOption { option: SearchOption::Wrap } => ActionType::SearchToggleWrap,
            Action::SearchToggleOption { option: SearchOption::WholeWord } => ActionType::SearchToggleWholeWord,

            Action::NewPane { direction: Some(Direction::Down), .. } => ActionType::NewPaneDown,
            Action::NewPane { direction: Some(Direction::Right), .. } => ActionType::NewPaneRight,
            Action::NewPane { direction: Some(_), .. } => ActionType::NewPaneDown, // fallback
            Action::NewPane { direction: None, .. } => ActionType::NewPaneWithoutDirection,
            Action::NewStackedPane { .. } => ActionType::NewStackedPane,

            Action::MoveTab { direction: Direction::Left } => ActionType::MoveTabLeft,
            Action::MoveTab { direction: Direction::Right } => ActionType::MoveTabRight,
            // MoveTab Up/Down aren't typical but map to Left/Right as fallback
            Action::MoveTab { .. } => ActionType::MoveTabLeft,

            Action::GoToPreviousTab => ActionType::GoToPreviousTab,
            Action::GoToNextTab => ActionType::GoToNextTab,
            Action::ScrollUp => ActionType::ScrollUp,
            Action::ScrollDown => ActionType::ScrollDown,
            Action::PageScrollUp => ActionType::PageScrollUp,
            Action::PageScrollDown => ActionType::PageScrollDown,
            Action::HalfPageScrollUp => ActionType::HalfPageScrollUp,
            Action::HalfPageScrollDown => ActionType::HalfPageScrollDown,

            Action::SwitchToMode { input_mode } => ActionType::SwitchToMode(*input_mode),
            Action::TogglePaneEmbedOrFloating => ActionType::TogglePaneEmbedOrFloating,
            Action::ToggleFocusFullscreen => ActionType::ToggleFocusFullscreen,
            Action::ToggleFloatingPanes => ActionType::ToggleFloatingPanes,
            Action::CloseFocus => ActionType::CloseFocus,
            Action::CloseTab => ActionType::CloseTab,
            Action::ToggleActiveSyncTab => ActionType::ToggleActiveSyncTab,
            Action::ToggleTab => ActionType::ToggleTab,
            Action::BreakPane => ActionType::BreakPane,
            Action::BreakPaneLeft => ActionType::BreakPaneLeft,
            Action::BreakPaneRight => ActionType::BreakPaneRight,
            Action::EditScrollback { .. } => ActionType::EditScrollback,
            Action::Detach => ActionType::Detach,
            Action::Quit => ActionType::Quit,
            action if action.launches_plugin("session-manager") => ActionType::SessionManager,
            action if action.launches_plugin("configuration") => ActionType::Configuration,
            action if action.launches_plugin("plugin-manager") => ActionType::PluginManager,
            action if is_any_plugin_launch(action) => {
                ActionType::LaunchPlugin(extract_plugin_name(action))
            }
            action if matches!(action, Action::NewTab { .. }) => ActionType::NewTab,
            _ => ActionType::Other(format!("{:?}", action)),
        }
    }
}

/// Returns true if the action launches any plugin (known or unknown).
pub fn is_any_plugin_launch(action: &Action) -> bool {
    let s = format!("{:?}", action);
    s.starts_with("LaunchOrFocusPlugin") || s.starts_with("LaunchPlugin(")
}

/// Extract a canonical plugin name string from a plugin-launch action.
pub fn extract_plugin_name(action: &Action) -> String {
    let s = format!("{:?}", action);
    if let Some(after) = s.split("name: \"").nth(1) {
        if let Some(name) = after.split('"').next() {
            return name.to_string();
        }
    }
    if let Some(after) = s.split("Zellij(PluginTag(\"").nth(1) {
        if let Some(tag) = after.split('"').next() {
            return format!("zellij:{}", tag);
        }
    }
    if let Some(after) = s.split("File(\"").nth(1) {
        if let Some(path) = after.split('"').next() {
            let name = path.split('/').last().unwrap_or(path);
            return name.trim_end_matches(".wasm").to_string();
        }
    }
    "Plugin".to_string()
}

/// Convert a plugin name to a human-readable display label.
fn plugin_display_name(name: &str) -> String {
    if let Some(tag) = name.strip_prefix("zellij:") {
        let mut chars = tag.chars();
        match chars.next() {
            None => tag.to_string(),
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
        }
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use zellij_tile::prelude::actions::{Action, SearchDirection, SearchOption};

    /// Every InputMode variant. The trailing match is for exhaustiveness only:
    /// adding a new InputMode without appending it to the list will fail to
    /// compile here.
    fn all_input_modes() -> Vec<InputMode> {
        let modes = vec![
            InputMode::Locked, InputMode::Normal, InputMode::Pane, InputMode::Tab,
            InputMode::Resize, InputMode::Move, InputMode::Scroll, InputMode::Search,
            InputMode::EnterSearch, InputMode::Session, InputMode::RenameTab,
            InputMode::RenamePane, InputMode::Tmux, InputMode::Prompt,
        ];
        let _exhaustive = |m: InputMode| match m {
            InputMode::Locked | InputMode::Normal | InputMode::Pane | InputMode::Tab
            | InputMode::Resize | InputMode::Move | InputMode::Scroll | InputMode::Search
            | InputMode::EnterSearch | InputMode::Session | InputMode::RenameTab
            | InputMode::RenamePane | InputMode::Tmux | InputMode::Prompt => (),
        };
        modes
    }

    /// Every ActionType variant (one of each "shape").  The trailing match is
    /// for exhaustiveness: a new variant added to the enum without being
    /// appended here will fail to compile.
    fn all_action_types() -> Vec<ActionType> {
        let _exhaustive = |a: ActionType| match a {
            ActionType::SwitchToMode(_) => (),
            ActionType::MoveFocusLeft | ActionType::MoveFocusDown | ActionType::MoveFocusUp | ActionType::MoveFocusRight => (),
            ActionType::GoToPreviousTab | ActionType::GoToNextTab | ActionType::ToggleTab => (),
            ActionType::ScrollDown | ActionType::ScrollUp | ActionType::HalfPageScrollDown | ActionType::HalfPageScrollUp | ActionType::PageScrollDown | ActionType::PageScrollUp => (),
            ActionType::SearchDown | ActionType::SearchUp | ActionType::SearchToggleCaseSensitivity | ActionType::SearchToggleWrap | ActionType::SearchToggleWholeWord => (),
            ActionType::NewPaneWithoutDirection | ActionType::NewPaneDown | ActionType::NewPaneRight | ActionType::NewStackedPane | ActionType::NewTab => (),
            ActionType::MovePaneLeft | ActionType::MovePaneDown | ActionType::MovePaneUp | ActionType::MovePaneRight => (),
            ActionType::ResizeIncreaseAll | ActionType::ResizeDecreaseAll => (),
            ActionType::ResizeIncreaseLeft | ActionType::ResizeIncreaseDown | ActionType::ResizeIncreaseUp | ActionType::ResizeIncreaseRight => (),
            ActionType::ResizeDecreaseLeft | ActionType::ResizeDecreaseDown | ActionType::ResizeDecreaseUp | ActionType::ResizeDecreaseRight => (),
            ActionType::MoveTabLeft | ActionType::MoveTabRight => (),
            ActionType::BreakPane | ActionType::BreakPaneLeft | ActionType::BreakPaneRight => (),
            ActionType::ToggleFocusFullscreen | ActionType::ToggleFloatingPanes | ActionType::TogglePaneEmbedOrFloating | ActionType::ToggleActiveSyncTab => (),
            ActionType::CloseFocus | ActionType::CloseTab | ActionType::Detach | ActionType::Quit => (),
            ActionType::EditScrollback => (),
            ActionType::SessionManager | ActionType::PluginManager | ActionType::Configuration | ActionType::LaunchPlugin(_) => (),
            ActionType::Other(_) => (),
        };

        let mut v = Vec::new();
        for m in all_input_modes() {
            v.push(ActionType::SwitchToMode(m));
        }
        v.extend([
            ActionType::MoveFocusLeft, ActionType::MoveFocusDown, ActionType::MoveFocusUp, ActionType::MoveFocusRight,
            ActionType::GoToPreviousTab, ActionType::GoToNextTab, ActionType::ToggleTab,
            ActionType::ScrollDown, ActionType::ScrollUp,
            ActionType::HalfPageScrollDown, ActionType::HalfPageScrollUp,
            ActionType::PageScrollDown, ActionType::PageScrollUp,
            ActionType::SearchDown, ActionType::SearchUp,
            ActionType::SearchToggleCaseSensitivity, ActionType::SearchToggleWrap, ActionType::SearchToggleWholeWord,
            ActionType::NewPaneWithoutDirection, ActionType::NewPaneDown, ActionType::NewPaneRight,
            ActionType::NewStackedPane, ActionType::NewTab,
            ActionType::MovePaneLeft, ActionType::MovePaneDown, ActionType::MovePaneUp, ActionType::MovePaneRight,
            ActionType::ResizeIncreaseAll, ActionType::ResizeDecreaseAll,
            ActionType::ResizeIncreaseLeft, ActionType::ResizeIncreaseDown, ActionType::ResizeIncreaseUp, ActionType::ResizeIncreaseRight,
            ActionType::ResizeDecreaseLeft, ActionType::ResizeDecreaseDown, ActionType::ResizeDecreaseUp, ActionType::ResizeDecreaseRight,
            ActionType::MoveTabLeft, ActionType::MoveTabRight,
            ActionType::BreakPane, ActionType::BreakPaneLeft, ActionType::BreakPaneRight,
            ActionType::ToggleFocusFullscreen, ActionType::ToggleFloatingPanes,
            ActionType::TogglePaneEmbedOrFloating, ActionType::ToggleActiveSyncTab,
            ActionType::CloseFocus, ActionType::CloseTab, ActionType::Detach, ActionType::Quit,
            ActionType::EditScrollback,
            ActionType::SessionManager, ActionType::PluginManager, ActionType::Configuration,
            ActionType::LaunchPlugin("zellij-loom".into()),
            ActionType::Other("NoOp".into()),
        ]);
        v
    }

    #[test]
    fn sort_key_is_unique_for_every_variant() {
        let mut groups: HashMap<u16, Vec<ActionType>> = HashMap::new();
        for a in all_action_types() {
            groups.entry(a.sort_key()).or_default().push(a);
        }
        let collisions: Vec<_> = groups
            .into_iter()
            .filter(|(_, v)| v.len() > 1)
            .collect();
        assert!(
            collisions.is_empty(),
            "sort_key collisions found: {:#?}",
            collisions,
        );
    }

    #[test]
    fn sort_key_buckets_match_their_documented_ranges() {
        // 0–19: mode switches; 20–39: navigation; 40–49: create;
        // 50–69: modify/move; 70–79: toggle; 80–89: close/exit;
        // 90–94: edit; 95–99: plugins; 100+: unknown.
        let bucket = |k: u16| -> &'static str {
            match k {
                0..=19 => "mode_switch",
                20..=39 => "navigation",
                40..=49 => "create",
                50..=69 => "modify_move",
                70..=79 => "toggle",
                80..=89 => "close_exit",
                90..=94 => "edit",
                95..=99 => "plugins",
                _ => "unknown",
            }
        };
        for a in all_action_types() {
            let expected = match &a {
                ActionType::SwitchToMode(_) => "mode_switch",
                ActionType::MoveFocusLeft | ActionType::MoveFocusDown | ActionType::MoveFocusUp | ActionType::MoveFocusRight
                | ActionType::GoToPreviousTab | ActionType::GoToNextTab | ActionType::ToggleTab
                | ActionType::ScrollDown | ActionType::ScrollUp
                | ActionType::HalfPageScrollDown | ActionType::HalfPageScrollUp
                | ActionType::PageScrollDown | ActionType::PageScrollUp
                | ActionType::SearchDown | ActionType::SearchUp
                | ActionType::SearchToggleCaseSensitivity | ActionType::SearchToggleWrap | ActionType::SearchToggleWholeWord => "navigation",
                ActionType::NewPaneWithoutDirection | ActionType::NewPaneDown | ActionType::NewPaneRight
                | ActionType::NewStackedPane | ActionType::NewTab => "create",
                ActionType::MovePaneLeft | ActionType::MovePaneDown | ActionType::MovePaneUp | ActionType::MovePaneRight
                | ActionType::ResizeIncreaseAll | ActionType::ResizeDecreaseAll
                | ActionType::ResizeIncreaseLeft | ActionType::ResizeIncreaseDown | ActionType::ResizeIncreaseUp | ActionType::ResizeIncreaseRight
                | ActionType::ResizeDecreaseLeft | ActionType::ResizeDecreaseDown | ActionType::ResizeDecreaseUp | ActionType::ResizeDecreaseRight
                | ActionType::MoveTabLeft | ActionType::MoveTabRight
                | ActionType::BreakPane | ActionType::BreakPaneLeft | ActionType::BreakPaneRight => "modify_move",
                ActionType::ToggleFocusFullscreen | ActionType::ToggleFloatingPanes
                | ActionType::TogglePaneEmbedOrFloating | ActionType::ToggleActiveSyncTab => "toggle",
                ActionType::CloseFocus | ActionType::CloseTab | ActionType::Detach | ActionType::Quit => "close_exit",
                ActionType::EditScrollback => "edit",
                ActionType::SessionManager | ActionType::PluginManager | ActionType::Configuration
                | ActionType::LaunchPlugin(_) => "plugins",
                ActionType::Other(_) => "unknown",
            };
            assert_eq!(
                bucket(a.sort_key()),
                expected,
                "{:?} (sort_key={}) fell outside its documented bucket",
                a,
                a.sort_key(),
            );
        }
    }

    #[test]
    fn from_action_classifies_directional_actions() {
        assert_eq!(
            ActionType::from_action(&Action::MoveFocus { direction: Direction::Left }),
            ActionType::MoveFocusLeft,
        );
        assert_eq!(
            ActionType::from_action(&Action::MovePane { direction: Some(Direction::Right) }),
            ActionType::MovePaneRight,
        );
        assert_eq!(
            ActionType::from_action(&Action::Resize {
                resize: Resize::Increase,
                direction: None,
            }),
            ActionType::ResizeIncreaseAll,
        );
        assert_eq!(
            ActionType::from_action(&Action::Resize {
                resize: Resize::Decrease,
                direction: Some(Direction::Up),
            }),
            ActionType::ResizeDecreaseUp,
        );
        assert_eq!(
            ActionType::from_action(&Action::MoveTab { direction: Direction::Right }),
            ActionType::MoveTabRight,
        );
    }

    #[test]
    fn from_action_classifies_search_and_navigation() {
        assert_eq!(
            ActionType::from_action(&Action::Search { direction: SearchDirection::Down }),
            ActionType::SearchDown,
        );
        assert_eq!(
            ActionType::from_action(&Action::SearchToggleOption {
                option: SearchOption::CaseSensitivity,
            }),
            ActionType::SearchToggleCaseSensitivity,
        );
        assert_eq!(ActionType::from_action(&Action::ScrollUp), ActionType::ScrollUp);
        assert_eq!(ActionType::from_action(&Action::HalfPageScrollDown), ActionType::HalfPageScrollDown);
        assert_eq!(ActionType::from_action(&Action::GoToPreviousTab), ActionType::GoToPreviousTab);
    }

    #[test]
    fn from_action_classifies_mode_switch() {
        assert_eq!(
            ActionType::from_action(&Action::SwitchToMode { input_mode: InputMode::Pane }),
            ActionType::SwitchToMode(InputMode::Pane),
        );
    }

    #[test]
    fn from_action_falls_back_to_other_for_unrecognized() {
        // NoOp has no dedicated ActionType branch, so it should land in Other(_).
        let result = ActionType::from_action(&Action::NoOp);
        assert!(matches!(result, ActionType::Other(_)));
    }

    #[test]
    fn plugin_display_name_strips_zellij_prefix_and_capitalizes() {
        assert_eq!(plugin_display_name("zellij:about"), "About");
        assert_eq!(plugin_display_name("zellij:share"), "Share");
        assert_eq!(plugin_display_name("zellij-loom"), "zellij-loom");
        assert_eq!(plugin_display_name(""), "");
    }
}
