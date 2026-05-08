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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_from_str_parses_every_supported_mode() {
        let cases = [
            ("Locked", InputMode::Locked),
            ("Normal", InputMode::Normal),
            ("Pane", InputMode::Pane),
            ("Tab", InputMode::Tab),
            ("Resize", InputMode::Resize),
            ("Move", InputMode::Move),
            ("Scroll", InputMode::Scroll),
            ("Search", InputMode::Search),
            ("EnterSearch", InputMode::EnterSearch),
            ("RenameTab", InputMode::RenameTab),
            ("RenamePane", InputMode::RenamePane),
            ("Session", InputMode::Session),
            ("Prompt", InputMode::Prompt),
            ("Tmux", InputMode::Tmux),
        ];
        for (s, expected) in cases {
            assert_eq!(mode_from_str(s), Some(expected), "input {s:?}");
        }
    }

    #[test]
    fn mode_from_str_round_trips_with_debug_format() {
        // `mode_from_str` is the inverse of `format!("{mode:?}")`, which is
        // what zellij exposes in config files. Pin that contract so renaming
        // an InputMode variant upstream surfaces here.
        for mode in [
            InputMode::Normal,
            InputMode::Locked,
            InputMode::Pane,
            InputMode::Tab,
            InputMode::Resize,
            InputMode::Move,
            InputMode::Scroll,
            InputMode::Search,
            InputMode::EnterSearch,
            InputMode::RenameTab,
            InputMode::RenamePane,
            InputMode::Session,
            InputMode::Prompt,
            InputMode::Tmux,
        ] {
            let dbg = format!("{:?}", mode);
            assert_eq!(mode_from_str(&dbg), Some(mode), "round-trip for {dbg}");
        }
    }

    #[test]
    fn mode_from_str_rejects_garbage() {
        assert_eq!(mode_from_str(""), None);
        assert_eq!(mode_from_str("normal"), None); // case-sensitive
        assert_eq!(mode_from_str("BogusMode"), None);
        assert_eq!(mode_from_str(" Normal"), None); // no whitespace tolerance
    }
}
