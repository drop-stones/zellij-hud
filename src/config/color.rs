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

/// Convert a 256-color terminal index to its standard RGB representation.
/// 0–15 use xterm defaults; 16–231 are the 6×6×6 color cube; 232–255 are the
/// grayscale ramp. Lets `dim_color` / `lighten_color` operate uniformly in RGB
/// so surface/dim derivations don't collapse to a single index for 8-bit
/// palettes.
fn eightbit_to_rgb(n: u8) -> (u8, u8, u8) {
    match n {
        // System 16 colors (xterm defaults).
        0 => (0, 0, 0),
        1 => (128, 0, 0),
        2 => (0, 128, 0),
        3 => (128, 128, 0),
        4 => (0, 0, 128),
        5 => (128, 0, 128),
        6 => (0, 128, 128),
        7 => (192, 192, 192),
        8 => (128, 128, 128),
        9 => (255, 0, 0),
        10 => (0, 255, 0),
        11 => (255, 255, 0),
        12 => (0, 0, 255),
        13 => (255, 0, 255),
        14 => (0, 255, 255),
        15 => (255, 255, 255),
        // 6×6×6 color cube (indices 16–231).
        16..=231 => {
            let i = n - 16;
            let scale = |c: u8| if c == 0 { 0 } else { 55 + 40 * c };
            (scale(i / 36), scale((i % 36) / 6), scale(i % 6))
        }
        // Grayscale ramp (indices 232–255).
        232..=255 => {
            let v = 8 + 10 * (n - 232);
            (v, v, v)
        }
    }
}

fn to_rgb(color: PaletteColor) -> (u8, u8, u8) {
    match color {
        PaletteColor::Rgb((r, g, b)) => (r, g, b),
        PaletteColor::EightBit(n) => eightbit_to_rgb(n),
    }
}

/// Produce a dimmed variant of a color (50% brightness). EightBit inputs are
/// converted to RGB first so the result preserves the hue instead of
/// collapsing to a single fallback index.
pub(crate) fn dim_color(color: PaletteColor) -> PaletteColor {
    let (r, g, b) = to_rgb(color);
    PaletteColor::Rgb((r / 2, g / 2, b / 2))
}

/// Lighten a color by adding `amount` to each RGB channel (clamped to 255).
/// EightBit inputs are converted to RGB first; otherwise surface and
/// surface_bright would resolve to the same index regardless of `amount`.
pub(crate) fn lighten_color(color: PaletteColor, amount: u8) -> PaletteColor {
    let (r, g, b) = to_rgb(color);
    PaletteColor::Rgb((
        r.saturating_add(amount),
        g.saturating_add(amount),
        b.saturating_add(amount),
    ))
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
    fn dim_color_converts_eightbit_via_rgb_table() {
        // EightBit(220) is in the 6×6×6 cube: index 204 = (5,4,0) → (255,215,0).
        // Dimming halves each channel.
        match dim_color(PaletteColor::EightBit(220)) {
            PaletteColor::Rgb((127, 107, 0)) => (),
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
    fn lighten_color_converts_eightbit_via_rgb_table() {
        // EightBit(232) is the darkest grayscale ramp entry → (8, 8, 8).
        // Lightening by 10 adds 10 to each channel.
        match lighten_color(PaletteColor::EightBit(232), 10) {
            PaletteColor::Rgb((18, 18, 18)) => (),
            other => panic!("got {other:?}"),
        }
    }

    #[test]
    fn lighten_color_distinguishes_surface_from_surface_bright_for_eightbit() {
        // Pin the system-theme invariant: surface = lighten(bg, +10) and
        // surface_bright = lighten(bg, +20) must differ when bg is EightBit,
        // even though the previous all-fallback-to-EightBit(8) behaviour
        // collapsed them.
        let bg = PaletteColor::EightBit(16);
        let surface = lighten_color(bg, 10);
        let surface_bright = lighten_color(bg, 20);
        assert_ne!(surface, surface_bright);
    }

    // ---- eightbit_to_rgb ----

    #[test]
    fn eightbit_to_rgb_named_16() {
        // The system 16 colors map to xterm defaults.
        assert_eq!(eightbit_to_rgb(0), (0, 0, 0));
        assert_eq!(eightbit_to_rgb(8), (128, 128, 128)); // "bright black"
        assert_eq!(eightbit_to_rgb(15), (255, 255, 255));
    }

    #[test]
    fn eightbit_to_rgb_color_cube() {
        // Index 16 is the cube origin (0,0,0); index 231 is the top corner.
        // Cube formula: 0 → 0, otherwise 55 + 40*c.
        assert_eq!(eightbit_to_rgb(16), (0, 0, 0));
        assert_eq!(eightbit_to_rgb(231), (255, 255, 255));
        // Mid-cube spot check: index 124 = 108 + 16 = (3, 0, 0) → (175, 0, 0).
        assert_eq!(eightbit_to_rgb(124), (175, 0, 0));
    }

    #[test]
    fn eightbit_to_rgb_grayscale_ramp() {
        // Ramp formula: 8 + 10 * (n - 232). Index 232 = 8, 255 = 238.
        assert_eq!(eightbit_to_rgb(232), (8, 8, 8));
        assert_eq!(eightbit_to_rgb(255), (238, 238, 238));
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
