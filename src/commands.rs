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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_escape_empties_become_explicit_empty_quotes() {
        // An empty arg in `sh -c` would otherwise vanish; force it to "''".
        assert_eq!(shell_escape(""), "''");
    }

    #[test]
    fn shell_escape_passes_through_safe_strings() {
        // Safe-character strings are returned borrowed (no allocation).
        for s in ["abc", "ABC", "a_b-c.d/e", "v1.2-rc3", "/home/user/.config"] {
            let out = shell_escape(s);
            assert_eq!(out, s);
            assert!(matches!(out, Cow::Borrowed(_)), "expected Borrowed for {s:?}");
        }
    }

    #[test]
    fn shell_escape_quotes_when_unsafe_characters_appear() {
        // Spaces, $, glob chars, etc. trigger quoting.
        assert_eq!(shell_escape("hello world"), "'hello world'");
        assert_eq!(shell_escape("$HOME"), "'$HOME'");
        assert_eq!(shell_escape("a*b"), "'a*b'");
    }

    #[test]
    fn shell_escape_escapes_internal_single_quotes() {
        // Standard POSIX trick: close-quote, escaped-quote, re-open-quote.
        assert_eq!(shell_escape("it's"), "'it'\\''s'");
        assert_eq!(shell_escape("'"), "''\\'''");
    }
}
