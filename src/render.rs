use zellij_hud::config::Color;
use zellij_hud::spans::resolve_and_emit;

use crate::State;

impl State {
    /// Render a format string (format_left or format_right) into an ANSI string.
    /// Uses the 2-pass pipeline: flatten into spans, then resolve positional
    /// color refs and emit ANSI.
    pub(crate) fn render_format(&self, format_str: &str, bar_bg: &Color) -> String {
        let mut spans = self.flatten_format(format_str);
        resolve_and_emit(&mut spans, bar_bg)
    }

    pub(crate) fn format_cwd(&self) -> String {
        // Show "~" when cwd is the user's home directory
        if let Ok(home) = std::env::var("HOME") {
            if self.cwd == std::path::PathBuf::from(&home) {
                return "~".to_string();
            }
        }
        if let Some(name) = self.cwd.file_name() {
            name.to_string_lossy().to_string()
        } else {
            self.cwd.to_string_lossy().to_string()
        }
    }
}
