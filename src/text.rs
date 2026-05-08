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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_len_counts_plain_ascii() {
        assert_eq!(visible_len(""), 0);
        assert_eq!(visible_len("hello"), 5);
        assert_eq!(visible_len("a b c"), 5);
    }

    #[test]
    fn visible_len_strips_color_escapes() {
        // \x1b[31m red \x1b[0m
        assert_eq!(visible_len("\x1b[31mred\x1b[0m"), 3);
        // 24-bit color
        assert_eq!(
            visible_len("\x1b[38;2;255;0;0mhi\x1b[0m"),
            2,
        );
        // 8-bit fg + bg combined
        assert_eq!(
            visible_len("\x1b[48;5;234m\x1b[38;5;255mok\x1b[0m"),
            2,
        );
    }

    #[test]
    fn visible_len_handles_wide_glyphs() {
        // CJK takes 2 columns each.
        assert_eq!(visible_len("漢字"), 4);
        // Mixed narrow + wide.
        assert_eq!(visible_len("a漢b"), 4);
    }

    #[test]
    fn visible_len_handles_nerd_font_icons() {
        // Nerd font icons live in the Unicode Private Use Area, which
        // `unicode-width` reports as width 1. Most terminals also render them
        // single-width because the font ships them as such, so this matches
        // what ends up on screen. Pin the value to catch any future width
        // regression.
        // 󰍀 (U+F0340) — used as the NORMAL mode icon.
        assert_eq!(visible_len("󰍀 NORMAL"), 8);
    }

    #[test]
    fn visible_len_treats_unterminated_escape_as_swallowed() {
        // Escape state ends only at `m`; an unterminated escape consumes
        // every following byte. This mirrors the actual rendered output:
        // an unterminated escape would also leave the terminal confused,
        // so swallowing it in the width count is fine.
        assert_eq!(visible_len("\x1b[31plain"), 0);
    }
}
