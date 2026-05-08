use std::borrow::Cow;

/// Context key for user-defined command widgets. Value = widget name.
pub const CMD_CONTEXT_USER: &str = "cmd_widget";

/// Output captured from a command widget execution.
#[derive(Clone, Default)]
pub struct CommandOutput {
    pub stdout: String,
    pub exit_code: i32,
}

/// Single-quote a string for safe use in `sh -c` commands.
///
/// Strings made up of unambiguously safe shell characters
/// (`[a-zA-Z0-9_\-./]`) are returned as-is; everything else is wrapped in
/// single quotes with internal `'` escaped as `'\''`.
pub fn shell_escape(s: &str) -> Cow<'_, str> {
    if s.is_empty() {
        return Cow::Borrowed("''");
    }
    if s.bytes().all(|b| matches!(b, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-' | b'.' | b'/')) {
        Cow::Borrowed(s)
    } else {
        Cow::Owned(format!("'{}'", s.replace('\'', "'\\''")))
    }
}
