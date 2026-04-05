use unicode_width::UnicodeWidthChar;

use crate::config::Color;
use crate::spans::resolve_and_emit;
use crate::State;

/// Count visible display width of a string, ignoring ANSI escape sequences
/// and accounting for wide characters (CJK, nerd font icons, emoji).
pub(crate) fn visible_len(s: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;
    for ch in s.chars() {
        if ch == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if ch == 'm' {
                in_escape = false;
            }
        } else {
            len += UnicodeWidthChar::width(ch).unwrap_or(0);
        }
    }
    len
}

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
