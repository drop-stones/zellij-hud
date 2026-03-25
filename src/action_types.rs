use zellij_tile::prelude::actions::Action;
use zellij_tile::prelude::*;

use crate::config::IconColors;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum ActionType {
    MoveFocus,
    MovePaneWithDirection,
    ResizeIncrease,
    ResizeDecrease,
    ResizeAny,
    Search,
    NewPaneDown,
    NewPaneRight,
    NewPaneWithoutDirection,
    GoToAdjacentTab,
    Scroll,
    PageScroll,
    HalfPageScroll,
    SessionManager,
    Configuration,
    PluginManager,
    LaunchPlugin(String),
    SwitchToMode(InputMode),
    TogglePaneEmbedOrFloating,
    ToggleFocusFullscreen,
    ToggleFloatingPanes,
    CloseFocus,
    CloseTab,
    ToggleActiveSyncTab,
    ToggleTab,
    BreakPane,
    BreakPaneLeftOrRight,
    EditScrollback,
    NewTab,
    NewStackedPane,
    Detach,
    Quit,
    Other(String),
}

impl ActionType {
    pub(crate) fn description(&self) -> String {
        match self {
            ActionType::MoveFocus => "Move focus".to_string(),
            ActionType::MovePaneWithDirection => "Move pane".to_string(),
            ActionType::ResizeIncrease => "Increase size".to_string(),
            ActionType::ResizeDecrease => "Decrease size".to_string(),
            ActionType::ResizeAny => "Resize".to_string(),
            ActionType::Search => "Search".to_string(),
            ActionType::NewPaneDown => "Horizontal split".to_string(),
            ActionType::NewPaneRight => "Vertical split".to_string(),
            ActionType::NewPaneWithoutDirection => "New pane".to_string(),
            ActionType::GoToAdjacentTab => "Move tab focus".to_string(),
            ActionType::Scroll => "Scroll".to_string(),
            ActionType::PageScroll => "Page scroll".to_string(),
            ActionType::HalfPageScroll => "Half page scroll".to_string(),
            ActionType::SessionManager => "Session manager".to_string(),
            ActionType::Configuration => "Configuration".to_string(),
            ActionType::PluginManager => "Plugin manager".to_string(),
            ActionType::LaunchPlugin(name) => plugin_display_name(name),
            ActionType::SwitchToMode(m) if *m == InputMode::RenamePane => "+rename-pane".to_string(),
            ActionType::SwitchToMode(m) if *m == InputMode::RenameTab => "+rename-tab".to_string(),
            ActionType::SwitchToMode(m) if *m == InputMode::EnterSearch => "+search".to_string(),
            ActionType::SwitchToMode(m) if *m == InputMode::Locked => "+locked".to_string(),
            ActionType::SwitchToMode(m) if *m == InputMode::Normal => "+normal".to_string(),
            ActionType::SwitchToMode(m) => format!("+{}", format!("{:?}", m).to_lowercase()),
            ActionType::TogglePaneEmbedOrFloating => "Float/embed".to_string(),
            ActionType::ToggleFocusFullscreen => "Fullscreen".to_string(),
            ActionType::ToggleFloatingPanes => "Toggle floating".to_string(),
            ActionType::CloseFocus => "Close pane".to_string(),
            ActionType::CloseTab => "Close tab".to_string(),
            ActionType::ToggleActiveSyncTab => "Sync tab".to_string(),
            ActionType::ToggleTab => "Circle tab".to_string(),
            ActionType::BreakPane => "Break pane".to_string(),
            ActionType::BreakPaneLeftOrRight => "Break to adjacent".to_string(),
            ActionType::EditScrollback => "Edit scrollback".to_string(),
            ActionType::NewTab => "New tab".to_string(),
            ActionType::NewStackedPane => "Stacked pane".to_string(),
            ActionType::Detach => "Detach".to_string(),
            ActionType::Quit => "Quit".to_string(),
            ActionType::Other(_) => "Other".to_string(),
        }
    }

    /// Whether this action switches to another input mode.
    pub(crate) fn is_mode_switch(&self) -> bool {
        matches!(self, ActionType::SwitchToMode(_))
    }

    /// Icon for the action type.
    /// Mode switch icons match status bar mode_icon() in render.rs.
    pub(crate) fn icon(&self) -> &str {
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
            ActionType::MoveFocus => "󰁌",
            ActionType::MovePaneWithDirection => "󰁌",
            ActionType::ResizeIncrease | ActionType::ResizeDecrease | ActionType::ResizeAny => "󰩨",
            ActionType::NewPaneDown | ActionType::NewPaneRight | ActionType::NewPaneWithoutDirection => "󰐕",
            ActionType::NewStackedPane => "󰐕",
            ActionType::CloseFocus | ActionType::CloseTab => "󰅖",
            ActionType::ToggleFocusFullscreen => "󰊓",
            ActionType::ToggleFloatingPanes | ActionType::TogglePaneEmbedOrFloating => "󰉈",
            ActionType::GoToAdjacentTab => "󰓩",
            ActionType::NewTab => "󰐕",
            ActionType::BreakPane | ActionType::BreakPaneLeftOrRight => "󰀞",
            ActionType::ToggleActiveSyncTab => "󰓦",
            ActionType::ToggleTab => "󰑍",
            ActionType::Scroll | ActionType::PageScroll | ActionType::HalfPageScroll => "󰠶",
            ActionType::Search => "󰍉",
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

    /// ANSI color escape for the icon, derived from the theme palette.
    pub(crate) fn icon_color<'a>(&self, colors: &'a IconColors) -> &'a str {
        match self {
            ActionType::SwitchToMode(_) => &colors.mode_switch,
            ActionType::MoveFocus | ActionType::MovePaneWithDirection => &colors.navigation,
            ActionType::GoToAdjacentTab | ActionType::ToggleTab => &colors.navigation,
            ActionType::ResizeIncrease | ActionType::ResizeDecrease | ActionType::ResizeAny => {
                &colors.resize
            }
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
            ActionType::BreakPane | ActionType::BreakPaneLeftOrRight => &colors.resize,
            ActionType::Scroll | ActionType::PageScroll | ActionType::HalfPageScroll => {
                &colors.navigation
            }
            ActionType::Search | ActionType::EditScrollback => &colors.search,
            ActionType::SessionManager | ActionType::Configuration | ActionType::PluginManager => {
                &colors.create
            }
            ActionType::LaunchPlugin(_) => &colors.plugin,
            ActionType::Other(_) => &colors.dim,
        }
    }

    pub(crate) fn from_action(action: &Action) -> Self {
        match action {
            Action::MoveFocus { .. } => ActionType::MoveFocus,
            Action::MovePane { direction: Some(_) } => ActionType::MovePaneWithDirection,
            Action::Resize { resize: Resize::Increase, direction: Some(_) } => ActionType::ResizeIncrease,
            Action::Resize { resize: Resize::Decrease, direction: Some(_) } => ActionType::ResizeDecrease,
            Action::Resize { direction: None, .. } => ActionType::ResizeAny,
            Action::Search { .. } => ActionType::Search,
            Action::NewPane { direction: Some(Direction::Down), .. } => ActionType::NewPaneDown,
            Action::NewPane { direction: Some(Direction::Right), .. } => ActionType::NewPaneRight,
            Action::NewPane { direction: Some(_), .. } => ActionType::NewPaneDown, // fallback
            Action::NewPane { direction: None, .. } => ActionType::NewPaneWithoutDirection,
            Action::NewStackedPane { .. } => ActionType::NewStackedPane,
            Action::BreakPaneLeft | Action::BreakPaneRight => ActionType::BreakPaneLeftOrRight,
            Action::GoToPreviousTab | Action::GoToNextTab => ActionType::GoToAdjacentTab,
            Action::ScrollUp | Action::ScrollDown => ActionType::Scroll,
            Action::PageScrollUp | Action::PageScrollDown => ActionType::PageScroll,
            Action::HalfPageScrollUp | Action::HalfPageScrollDown => ActionType::HalfPageScroll,
            Action::SwitchToMode { input_mode } => ActionType::SwitchToMode(*input_mode),
            Action::TogglePaneEmbedOrFloating => ActionType::TogglePaneEmbedOrFloating,
            Action::ToggleFocusFullscreen => ActionType::ToggleFocusFullscreen,
            Action::ToggleFloatingPanes => ActionType::ToggleFloatingPanes,
            Action::CloseFocus => ActionType::CloseFocus,
            Action::CloseTab => ActionType::CloseTab,
            Action::ToggleActiveSyncTab => ActionType::ToggleActiveSyncTab,
            Action::ToggleTab => ActionType::ToggleTab,
            Action::BreakPane => ActionType::BreakPane,
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
pub(crate) fn is_any_plugin_launch(action: &Action) -> bool {
    let s = format!("{:?}", action);
    s.starts_with("LaunchOrFocusPlugin") || s.starts_with("LaunchPlugin(")
}

/// Extract a canonical plugin name string from a plugin-launch action.
/// Mirrors `RunPluginOrAlias::location_string()` / `RunPluginLocation::display()`.
pub(crate) fn extract_plugin_name(action: &Action) -> String {
    let s = format!("{:?}", action);
    // Alias variant: PluginAlias { name: "zellij-loom", ...
    if let Some(after) = s.split("name: \"").nth(1) {
        if let Some(name) = after.split('"').next() {
            return name.to_string();
        }
    }
    // RunPlugin with Zellij location: location: Zellij(PluginTag("about"))
    if let Some(after) = s.split("Zellij(PluginTag(\"").nth(1) {
        if let Some(tag) = after.split('"').next() {
            return format!("zellij:{}", tag);
        }
    }
    // RunPlugin with file location: location: File("/path/to/plugin.wasm")
    if let Some(after) = s.split("File(\"").nth(1) {
        if let Some(path) = after.split('"').next() {
            let name = path.split('/').last().unwrap_or(path);
            return name.trim_end_matches(".wasm").to_string();
        }
    }
    "Plugin".to_string()
}

/// Convert a plugin name to a human-readable display label.
/// "zellij:about" → "About" (strip zellij: prefix, capitalize)
/// "zellij-loom" → "zellij-loom" (third-party alias: keep as-is)
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
