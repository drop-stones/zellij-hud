//! Small text utilities shared by the bar/tooltip layout code.

use unicode_width::UnicodeWidthChar;

/// Count visible display width of a string, ignoring ANSI escape sequences
/// and accounting for wide characters (CJK, nerd font icons, emoji).
pub fn visible_len(s: &str) -> usize {
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
