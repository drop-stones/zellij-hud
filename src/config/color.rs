//! Color enum and palette-color helpers.
//!
//! `Color` is the terminal color representation used throughout the HUD
//! (resolved from palette names / hex / 8-bit specs at parse time, then
//! emitted as ANSI escapes at render time). The `dim_color` / `lighten_color`
//! / `palette_color_to_hex` helpers convert from zellij's `PaletteColor` —
//! used by the system theme path to derive surface/dim colors from the
//! terminal's own palette.

use zellij_tile::prelude::PaletteColor;

/// RGB or 8-bit terminal color, used throughout the HUD for fg and bg rendering.
#[derive(Clone, Default)]
pub enum Color {
    #[default]
    None,
    Rgb(u8, u8, u8),
    EightBit(u8),
}

impl Color {
    /// ANSI foreground escape sequence.
    pub fn fg(&self) -> String {
        match self {
            Color::None => String::new(),
            Color::Rgb(r, g, b) => format!("\x1b[38;2;{};{};{}m", r, g, b),
            Color::EightBit(n) => format!("\x1b[38;5;{}m", n),
        }
    }
    /// ANSI background escape sequence.
    pub fn bg(&self) -> String {
        match self {
            Color::None => String::new(),
            Color::Rgb(r, g, b) => format!("\x1b[48;2;{};{};{}m", r, g, b),
            Color::EightBit(n) => format!("\x1b[48;5;{}m", n),
        }
    }
    /// Parse from `#RRGGBB` hex or `8bit:N` string.
    pub(crate) fn from_hex(hex: &str) -> Option<Self> {
        if let Some(n) = hex.strip_prefix("8bit:") {
            return Some(Color::EightBit(n.parse().ok()?));
        }
        let hex = hex.strip_prefix('#').unwrap_or(hex);
        if hex.len() != 6 { return None; }
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Color::Rgb(r, g, b))
    }
}

/// Produce a dimmed variant of a color (50% brightness).
pub(crate) fn dim_color(color: PaletteColor) -> PaletteColor {
    match color {
        PaletteColor::Rgb((r, g, b)) => PaletteColor::Rgb((r / 2, g / 2, b / 2)),
        // EightBit(8) = "bright black" = dark gray in most terminals
        PaletteColor::EightBit(_) => PaletteColor::EightBit(8),
    }
}

/// Lighten a color by adding `amount` to each RGB channel (clamped to 255).
pub(crate) fn lighten_color(color: PaletteColor, amount: u8) -> PaletteColor {
    match color {
        PaletteColor::Rgb((r, g, b)) => PaletteColor::Rgb((
            r.saturating_add(amount),
            g.saturating_add(amount),
            b.saturating_add(amount),
        )),
        PaletteColor::EightBit(_) => PaletteColor::EightBit(8),
    }
}

/// Convert a `PaletteColor` to a hex string usable by `Color::from_hex`.
/// Rgb → "#rrggbb", EightBit → "8bit:N".
pub(crate) fn palette_color_to_hex(color: PaletteColor) -> String {
    match color {
        PaletteColor::Rgb((r, g, b)) => format!("#{:02x}{:02x}{:02x}", r, g, b),
        PaletteColor::EightBit(n) => format!("8bit:{}", n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Color::fg / bg ----

    #[test]
    fn color_none_emits_no_escape() {
        assert_eq!(Color::None.fg(), "");
        assert_eq!(Color::None.bg(), "");
    }

    #[test]
    fn color_rgb_emits_24bit_ansi() {
        assert_eq!(Color::Rgb(255, 0, 128).fg(), "\x1b[38;2;255;0;128m");
        assert_eq!(Color::Rgb(255, 0, 128).bg(), "\x1b[48;2;255;0;128m");
    }

    #[test]
    fn color_eightbit_emits_256_ansi() {
        assert_eq!(Color::EightBit(123).fg(), "\x1b[38;5;123m");
        assert_eq!(Color::EightBit(0).bg(), "\x1b[48;5;0m");
    }

    // ---- Color::from_hex ----

    #[test]
    fn from_hex_parses_rgb_with_and_without_leading_hash() {
        // Hex is normalised to RGB; both "#abcdef" and "abcdef" work so users
        // can write either in KDL.
        match Color::from_hex("#7aa2f7") {
            Some(Color::Rgb(0x7a, 0xa2, 0xf7)) => (),
            other => panic!("expected Rgb, got {:?}", color_label(&other)),
        }
        match Color::from_hex("7aa2f7") {
            Some(Color::Rgb(0x7a, 0xa2, 0xf7)) => (),
            other => panic!("expected Rgb, got {:?}", color_label(&other)),
        }
    }

    #[test]
    fn from_hex_parses_eight_bit_prefix() {
        match Color::from_hex("8bit:33") {
            Some(Color::EightBit(33)) => (),
            other => panic!("expected EightBit(33), got {:?}", color_label(&other)),
        }
        match Color::from_hex("8bit:0") {
            Some(Color::EightBit(0)) => (),
            other => panic!("expected EightBit(0), got {:?}", color_label(&other)),
        }
    }

    #[test]
    fn from_hex_rejects_malformed_inputs() {
        assert!(matches!(Color::from_hex(""), None));
        assert!(matches!(Color::from_hex("#abc"), None)); // 3-digit shorthand not supported
        assert!(matches!(Color::from_hex("#abcdefg"), None)); // wrong length
        assert!(matches!(Color::from_hex("#zzzzzz"), None)); // non-hex
        assert!(matches!(Color::from_hex("8bit:256"), None)); // out of range u8
        assert!(matches!(Color::from_hex("8bit:abc"), None)); // non-numeric
    }

    // ---- dim_color ----

    #[test]
    fn dim_color_halves_rgb_channels() {
        match dim_color(PaletteColor::Rgb((200, 100, 50))) {
            PaletteColor::Rgb((100, 50, 25)) => (),
            other => panic!("got {other:?}"),
        }
    }

    #[test]
    fn dim_color_collapses_eightbit_to_dark_gray() {
        // 8-bit input has no per-channel granularity, so we fall back to
        // EightBit(8) (terminal "bright black"). Pin the exact value so a
        // future change is intentional.
        match dim_color(PaletteColor::EightBit(220)) {
            PaletteColor::EightBit(8) => (),
            other => panic!("got {other:?}"),
        }
    }

    // ---- lighten_color ----

    #[test]
    fn lighten_color_adds_per_channel_with_saturation() {
        match lighten_color(PaletteColor::Rgb((100, 200, 250)), 20) {
            PaletteColor::Rgb((120, 220, 255)) => (),
            other => panic!("got {other:?}"),
        }
        // Saturation: every channel clamps at 255.
        match lighten_color(PaletteColor::Rgb((250, 250, 250)), 100) {
            PaletteColor::Rgb((255, 255, 255)) => (),
            other => panic!("got {other:?}"),
        }
    }

    #[test]
    fn lighten_color_collapses_eightbit_to_dark_gray() {
        // Same EightBit limitation as dim_color; pin the fallback.
        match lighten_color(PaletteColor::EightBit(220), 10) {
            PaletteColor::EightBit(8) => (),
            other => panic!("got {other:?}"),
        }
    }

    // ---- palette_color_to_hex ----

    #[test]
    fn palette_color_to_hex_emits_lowercase_six_digit_hex() {
        assert_eq!(
            palette_color_to_hex(PaletteColor::Rgb((0x7a, 0xa2, 0xf7))),
            "#7aa2f7",
        );
        // Single-digit channels are zero-padded.
        assert_eq!(
            palette_color_to_hex(PaletteColor::Rgb((0, 1, 15))),
            "#00010f",
        );
    }

    #[test]
    fn palette_color_to_hex_round_trips_with_from_hex() {
        let original = PaletteColor::Rgb((0x12, 0x34, 0x56));
        let hex = palette_color_to_hex(original);
        match Color::from_hex(&hex) {
            Some(Color::Rgb(0x12, 0x34, 0x56)) => (),
            other => panic!("round-trip failed for {hex}: {:?}", color_label(&other)),
        }
    }

    #[test]
    fn palette_color_to_hex_emits_eight_bit_prefix() {
        assert_eq!(palette_color_to_hex(PaletteColor::EightBit(33)), "8bit:33");
    }

    fn color_label(c: &Option<Color>) -> &'static str {
        match c {
            None => "None",
            Some(Color::None) => "Some(Color::None)",
            Some(Color::Rgb(_, _, _)) => "Some(Color::Rgb)",
            Some(Color::EightBit(_)) => "Some(Color::EightBit)",
        }
    }
}
