//! Helpers for parsing/identifying zellij `InputMode` values.

use zellij_tile::prelude::InputMode;

/// Parse an InputMode from its Debug string representation (e.g. "Normal", "Pane").
///
/// Used to read `initial_mode` out of plugin configuration.
pub fn mode_from_str(s: &str) -> Option<InputMode> {
    match s {
        "Locked" => Some(InputMode::Locked),
        "Normal" => Some(InputMode::Normal),
        "Pane" => Some(InputMode::Pane),
        "Tab" => Some(InputMode::Tab),
        "Resize" => Some(InputMode::Resize),
        "Move" => Some(InputMode::Move),
        "Scroll" => Some(InputMode::Scroll),
        "Search" => Some(InputMode::Search),
        "EnterSearch" => Some(InputMode::EnterSearch),
        "RenameTab" => Some(InputMode::RenameTab),
        "RenamePane" => Some(InputMode::RenamePane),
        "Session" => Some(InputMode::Session),
        "Prompt" => Some(InputMode::Prompt),
        "Tmux" => Some(InputMode::Tmux),
        _ => None,
    }
}
