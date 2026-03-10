use zellij_tile::prelude::actions::Action;
use zellij_tile::prelude::*;

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
                InputMode::Normal => "󰰓",
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
            ActionType::Detach => "󰗼",
            ActionType::Quit => "󰈆",
            ActionType::Other(_) => "󰘳",
        }
    }

    /// ANSI color escape for the icon (tokyonight defaults).
    pub(crate) fn icon_color(&self) -> &str {
        match self {
            // Mode switch: blue #7aa2f7
            ActionType::SwitchToMode(_) => "\x1b[38;2;122;162;247m",
            // Navigation/move: cyan #2ac3de
            ActionType::MoveFocus | ActionType::MovePaneWithDirection => "\x1b[38;2;42;195;222m",
            ActionType::GoToAdjacentTab | ActionType::ToggleTab => "\x1b[38;2;42;195;222m",
            // Resize: orange #ff9e64
            ActionType::ResizeIncrease | ActionType::ResizeDecrease | ActionType::ResizeAny => {
                "\x1b[38;2;255;158;100m"
            }
            // Create/new: green #9ece6a
            ActionType::NewPaneDown
            | ActionType::NewPaneRight
            | ActionType::NewPaneWithoutDirection
            | ActionType::NewStackedPane
            | ActionType::NewTab => "\x1b[38;2;158;206;106m",
            // Close/quit/detach: red #f7768e
            ActionType::CloseFocus | ActionType::CloseTab | ActionType::Quit => {
                "\x1b[38;2;247;118;142m"
            }
            ActionType::Detach => "\x1b[38;2;247;118;142m",
            // Toggle: yellow #e0af68
            ActionType::ToggleFocusFullscreen
            | ActionType::ToggleFloatingPanes
            | ActionType::TogglePaneEmbedOrFloating
            | ActionType::ToggleActiveSyncTab => "\x1b[38;2;224;175;104m",
            // Break pane: orange #ff9e64
            ActionType::BreakPane | ActionType::BreakPaneLeftOrRight => {
                "\x1b[38;2;255;158;100m"
            }
            // Scroll: cyan #2ac3de
            ActionType::Scroll | ActionType::PageScroll | ActionType::HalfPageScroll => {
                "\x1b[38;2;42;195;222m"
            }
            // Search/edit: purple #bb9af7
            ActionType::Search | ActionType::EditScrollback => "\x1b[38;2;187;154;247m",
            // Session/config/plugin: green #9ece6a
            ActionType::SessionManager | ActionType::Configuration | ActionType::PluginManager => {
                "\x1b[38;2;158;206;106m"
            }
            // Fallback: dim #565f89
            ActionType::Other(_) => "\x1b[38;2;86;95;137m",
        }
    }

    pub(crate) fn from_action(action: &Action) -> Self {
        match action {
            Action::MoveFocus(_) => ActionType::MoveFocus,
            Action::MovePane(Some(_)) => ActionType::MovePaneWithDirection,
            Action::Resize(Resize::Increase, Some(_)) => ActionType::ResizeIncrease,
            Action::Resize(Resize::Decrease, Some(_)) => ActionType::ResizeDecrease,
            Action::Resize(_, None) => ActionType::ResizeAny,
            Action::Search(_) => ActionType::Search,
            Action::NewPane(Some(Direction::Down), _, _) => ActionType::NewPaneDown,
            Action::NewPane(Some(Direction::Right), _, _) => ActionType::NewPaneRight,
            Action::NewPane(Some(_), _, _) => ActionType::NewPaneDown, // fallback
            Action::NewPane(None, _, _) => ActionType::NewPaneWithoutDirection,
            Action::NewStackedPane(_, _) => ActionType::NewStackedPane,
            Action::BreakPaneLeft | Action::BreakPaneRight => ActionType::BreakPaneLeftOrRight,
            Action::GoToPreviousTab | Action::GoToNextTab => ActionType::GoToAdjacentTab,
            Action::ScrollUp | Action::ScrollDown => ActionType::Scroll,
            Action::PageScrollUp | Action::PageScrollDown => ActionType::PageScroll,
            Action::HalfPageScrollUp | Action::HalfPageScrollDown => ActionType::HalfPageScroll,
            Action::SwitchToMode(m) => ActionType::SwitchToMode(*m),
            Action::TogglePaneEmbedOrFloating => ActionType::TogglePaneEmbedOrFloating,
            Action::ToggleFocusFullscreen => ActionType::ToggleFocusFullscreen,
            Action::ToggleFloatingPanes => ActionType::ToggleFloatingPanes,
            Action::CloseFocus => ActionType::CloseFocus,
            Action::CloseTab => ActionType::CloseTab,
            Action::ToggleActiveSyncTab => ActionType::ToggleActiveSyncTab,
            Action::ToggleTab => ActionType::ToggleTab,
            Action::BreakPane => ActionType::BreakPane,
            Action::EditScrollback => ActionType::EditScrollback,
            Action::Detach => ActionType::Detach,
            Action::Quit => ActionType::Quit,
            action if action.launches_plugin("session-manager") => ActionType::SessionManager,
            action if action.launches_plugin("configuration") => ActionType::Configuration,
            action if action.launches_plugin("plugin-manager") => ActionType::PluginManager,
            action if matches!(action, Action::NewTab(..)) => ActionType::NewTab,
            _ => ActionType::Other(format!("{:?}", action)),
        }
    }
}
