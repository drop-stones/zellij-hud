use std::collections::{BTreeMap, HashMap};

use zellij_tile::prelude::{InputMode, PaletteColor, Styling};

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
    fn from_hex(hex: &str) -> Option<Self> {
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

/// Per-widget style (v3 config). Color values are stored as raw strings
/// (palette names, hex, or "accent") and resolved at render time.
#[derive(Clone, Default)]
pub struct WidgetStyle {
    /// Foreground color: palette name, hex, or "accent".
    pub fg: String,
    /// Background color: palette name, hex, or "accent". Empty = no bg.
    pub bg: String,
    /// Text decorations: "bold", "italic", "bold,italic", or "".
    pub attr: String,
}

impl WidgetStyle {
    pub fn new(fg: &str, bg: &str, attr: &str) -> Self {
        Self {
            fg: fg.to_string(),
            bg: bg.to_string(),
            attr: attr.to_string(),
        }
    }
}

/// Per-widget style defaults: (fg, bg, attr).
type WStyle = (&'static str, &'static str, &'static str);

/// Style preset for the status bar and tooltip.
/// Provides default widget styles; user overrides apply on top.
struct StyleDefaults {
    format_left: &'static str,
    format_center: &'static str,
    format_right: &'static str,
    bar_bg: &'static str,
    mode_format: &'static str,
    mode_style: WStyle,
    /// Per-mode content overrides: (mode_suffix, content_text).
    /// mode_suffix corresponds to the suffix in `mode_content_{suffix}` config keys.
    /// Empty slice = use global defaults (icons + uppercase names).
    mode_content: &'static [(&'static str, &'static str)],
    session_format: &'static str,
    session_style: WStyle,
    tab_active_format: &'static str,
    tab_inactive_format: &'static str,
    tab_separator_content: &'static str,
    tab_separator_style: WStyle,
    tab_active_style: WStyle,
    tab_inactive_style: WStyle,
    tab_active_index_style: Option<WStyle>,
    tab_active_index_format: &'static str,
    tab_active_name_format: &'static str,
    tab_active_sync_style: Option<WStyle>,
    tab_active_sync_format: &'static str,
    tab_active_fullscreen_style: Option<WStyle>,
    tab_active_fullscreen_format: &'static str,
    tab_inactive_index_style: Option<WStyle>,
    tab_inactive_index_format: &'static str,
    tab_inactive_name_format: &'static str,
    tab_inactive_sync_format: &'static str,
    tab_inactive_fullscreen_format: &'static str,
    cwd_format: &'static str,
    cwd_style: WStyle,
    /// Per-built-in-command-widget style/format overrides.
    /// These override the fixed defaults when a style preset needs a different look.
    command_overrides: &'static [(&'static str, WStyle, &'static str)],
}

impl StyleDefaults {
    fn from_name(name: &str) -> Self {
        match name {
            //                          (fg,       bg,        attr)
            //
            // Powerline: per-segment bg with arrow separators.
            // Separator text widgets (s_ms, s_sb, etc.) are defined as
            // default text widgets in build_from_palette().
            //
            // Left:  [mode bg=accent]▶[session bg=dim]▶[tabs]
            // Right: [cwd]▸[git]◂[memory bg=dim]◂[time bg=accent]
            "powerline" => Self {
                format_left:   "{mode}{s_ms}{session}{s_sb}{tabs}",
                format_center: "",
                format_right:  "{cwd}{git_branch}{s_gm}{memory}{s_mt}{time}",
                bar_bg: "bg",
                mode_format:        " {content} ",
                mode_style:         ("bg",     "accent",  "bold"),
                mode_content:       &[],
                session_format:     " 󰆍 {name} ",
                session_style:      ("accent", "surface",  ""),
                tab_active_format:  "{ta_in} {name}{fullscreen_indicator}{sync_indicator} {ta_out}",
                tab_inactive_format: "{ti_in} {name}{fullscreen_indicator}{sync_indicator} {ti_out}",
                tab_separator_content:      "",
                tab_separator_style: ("dim",   "",  ""),
                tab_active_style:   ("fg",     "surface_bright", "bold"),
                tab_inactive_style: ("dim",    "surface", ""),
                tab_active_index_style: None,
                tab_active_index_format: "{content}",
                tab_active_name_format: "{content}",
                tab_active_sync_style: Some(("accent", "", "")),
                tab_active_sync_format: " {content}",
                tab_active_fullscreen_style: Some(("accent", "", "")),
                tab_active_fullscreen_format: " {content}",
                tab_inactive_index_style: None,
                tab_inactive_index_format: "{content}",
                tab_inactive_name_format: "{content}",
                tab_inactive_sync_format: " {content}",
                tab_inactive_fullscreen_format: " {content}",
                cwd_format:         " \u{f0256} {cwd} ",
                cwd_style:          ("cyan",   "",        ""),
                command_overrides: &[
                    ("git_branch", ("orange", "",        ""), "{s_cg} \u{e0a0} {stdout} "),
                    ("memory",     ("accent", "surface", ""), " \u{f035b} {stdout} "),
                    ("date",       ("bg",     "accent",  ""), " \u{f00ed} {stdout} "),
                    ("time",       ("bg",     "accent",  ""), " \u{f0954} {stdout} "),
                ],
            },
            //
            // Bubble: floating pill segments with two-tone icon badges.
            // Separator text widgets (pill_left, pill_right, gap, icons)
            // are defined as default text widgets in build_from_palette().
            //
            // Left:  [mode]╮ ╭ICON session╮ ╭IDX name╮ ╭IDX name╮
            // Right: ╭ICON cwd╮ ╭ICON git╮ ╭ICON mem╮ ╭ICON time╮
            "bubble" => Self {
                format_left:   "{mode}{pill_right}{gap}{pill_left}{session}{pill_right}{tabs}",
                format_center: "",
                format_right:  "{pill_left}{cwd}{pill_right} {git_branch}{pill_left}{memory}{pill_right} {pill_left}{time}{pill_right}",
                bar_bg: "bg",
                mode_format:        " {content}",
                mode_style:         ("bg",     "accent",       "bold"),
                mode_content:       &[],
                session_format:     "{sess_icon} {name}",
                session_style:      ("cyan",   "surface",      ""),
                tab_active_format:  "{gap}{pill_left}{index}{name}{fullscreen_indicator}{sync_indicator}{pill_right}",
                tab_inactive_format: "{gap}{pill_left}{index}{name}{fullscreen_indicator}{sync_indicator}{pill_right}",
                tab_separator_content:      "",
                tab_separator_style: ("dim",   "",  ""),
                tab_active_style:   ("fg",     "surface_bright", ""),
                tab_inactive_style: ("dim",    "surface",        ""),
                tab_active_index_style: Some(("bg", "blue", "bold")),
                tab_active_index_format: "{content} ",
                tab_active_name_format: " {content}",
                tab_active_sync_style: Some(("accent", "", "")),
                tab_active_sync_format: " {content} ",
                tab_active_fullscreen_style: Some(("accent", "", "")),
                tab_active_fullscreen_format: " {content} ",
                tab_inactive_index_style: Some(("bg", "dim", "")),
                tab_inactive_index_format: "{content} ",
                tab_inactive_name_format: " {content}",
                tab_inactive_sync_format: " {content} ",
                tab_inactive_fullscreen_format: " {content} ",
                cwd_format:         "{cwd_icon} {cwd}",
                cwd_style:          ("cyan",   "surface",      ""),
                command_overrides: &[
                    ("git_branch", ("magenta","surface",  ""), "{pill_left}{git_icon} {stdout}{pill_right}{gap}"),
                    ("memory",     ("green",  "surface",  ""), "{mem_icon} {stdout}"),
                    ("date",       ("magenta","surface",  ""), "{date_icon} {stdout}"),
                    ("time",       ("blue",   "surface",  ""), "{time_icon} {stdout}"),
                ],
            },
            //
            // Minimal: dotbar style — mode indicator left, tabs centered with
            // dot separators, time right. No icons, no segment backgrounds.
            //
            // Left:   󰍀 normal
            // Center: tab1 • tab2 • tab3
            // Right:  21:00
            "minimal" => Self {
                format_left:   "{mode}",
                format_center: "{tabs}",
                format_right:  "{time}",
                bar_bg: "",
                mode_format:        " {content} ",
                mode_style:         ("bg",     "accent",  ""),
                mode_content: &[
                    ("normal",       "\u{f0340} normal"),
                    ("locked",       "\u{f033e} locked"),
                    ("pane",         "\u{f0616} pane"),
                    ("tab",          "\u{f04e9} tab"),
                    ("resize",       "\u{f0a68} resize"),
                    ("move",         "\u{f01be} move"),
                    ("scroll",       "\u{f0836} scroll"),
                    ("search",       "\u{f0349} search"),
                    ("enter_search", "\u{f0349} search"),
                    ("rename_tab",   "\u{f03eb} rename tab"),
                    ("rename_pane",  "\u{f03eb} rename pane"),
                    ("session",      "\u{f10ac} session"),
                    ("prompt",       "\u{f0625} prompt"),
                    ("tmux",         "\u{f0c23} tmux"),
                ],
                session_format:     " {name} ",
                session_style:      ("dim",    "",        ""),
                tab_active_format:  "{name}{fullscreen_indicator}{sync_indicator}",
                tab_inactive_format: "{name}{fullscreen_indicator}{sync_indicator}",
                tab_separator_content:      " \u{2022} ",
                tab_separator_style: ("dim",   "",  ""),
                tab_active_style:   ("fg",     "",  "bold"),
                tab_inactive_style: ("dim",    "",  ""),
                tab_active_index_style: None,
                tab_active_index_format: "{content}",
                tab_active_name_format: "{content}",
                tab_active_sync_style: Some(("accent", "", "")),
                tab_active_sync_format: " {content}",
                tab_active_fullscreen_style: Some(("accent", "", "")),
                tab_active_fullscreen_format: " {content}",
                tab_inactive_index_style: None,
                tab_inactive_index_format: "{content}",
                tab_inactive_name_format: "{content}",
                tab_inactive_sync_format: " {content}",
                tab_inactive_fullscreen_format: " {content}",
                cwd_format:         " \u{f0256} {cwd} ",
                cwd_style:          ("cyan",   "",  ""),
                command_overrides: &[
                    ("time", ("dim", "", ""), " {stdout} "),
                ],
            },
            // "custom": blank slate — empty format strings, no text widgets,
            // only built-in widgets (mode, session, tabs, cwd) and
            // built-in command widgets (time, date, memory, git_branch).
            "custom" => Self {
                format_left:   "",
                format_center: "",
                format_right:  "",
                bar_bg: "bg",
                mode_format:        " {content} ",
                mode_style:         ("accent", "",  "bold"),
                mode_content:       &[],
                session_format:     " {name} ",
                session_style:      ("cyan",   "",  ""),
                tab_active_format:  " {name}{fullscreen_indicator}{sync_indicator}",
                tab_inactive_format: " {name}{fullscreen_indicator}{sync_indicator}",
                tab_separator_content:      "",
                tab_separator_style: ("dim",   "",  ""),
                tab_active_style:   ("fg",     "",  "bold"),
                tab_inactive_style: ("dim",    "",  ""),
                tab_active_index_style: None,
                tab_active_index_format: "{content}",
                tab_active_name_format: "{content}",
                tab_active_sync_style: Some(("accent", "", "")),
                tab_active_sync_format: " {content} ",
                tab_active_fullscreen_style: Some(("accent", "", "")),
                tab_active_fullscreen_format: " {content} ",
                tab_inactive_index_style: None,
                tab_inactive_index_format: "{content}",
                tab_inactive_name_format: "{content}",
                tab_inactive_sync_format: " {content} ",
                tab_inactive_fullscreen_format: " {content} ",
                cwd_format:         " {cwd} ",
                cwd_style:          ("cyan",   "",  ""),
                command_overrides:  &[],
            },
            // "simple" (default): flat look with thin separators and icons.
            // A single "sep" text widget is defined in build_from_palette().
            // git_branch includes a leading {sep} so the separator hides
            // when the widget is empty (not in a git repo).
            _ => Self {
                format_left:   "{mode}{sep}{session}{sep}{tabs}",
                format_center: "",
                format_right:  "{cwd}{git_branch}{sep}{memory}{sep}{time}",
                bar_bg: "bg",
                mode_format:        " {content} ",
                mode_style:         ("accent", "",  "bold"),
                mode_content:       &[],
                session_format:     " 󰆍 {name} ",
                session_style:      ("cyan",   "",  ""),
                tab_active_format:  " {name}{fullscreen_indicator}{sync_indicator}",
                tab_inactive_format: " {name}{fullscreen_indicator}{sync_indicator}",
                tab_separator_content:      "",
                tab_separator_style: ("dim",   "",  ""),
                tab_active_style:   ("fg",     "",  "bold"),
                tab_inactive_style: ("dim",    "",  ""),
                tab_active_index_style: None,
                tab_active_index_format: "{content}",
                tab_active_name_format: "{content}",
                tab_active_sync_style: Some(("accent", "", "")),
                tab_active_sync_format: " {content} ",
                tab_active_fullscreen_style: Some(("accent", "", "")),
                tab_active_fullscreen_format: " {content} ",
                tab_inactive_index_style: None,
                tab_inactive_index_format: "{content}",
                tab_inactive_name_format: "{content}",
                tab_inactive_sync_format: " {content} ",
                tab_inactive_fullscreen_format: " {content} ",
                cwd_format:         " \u{f0256} {cwd} ",
                cwd_style:          ("cyan",   "",  ""),
                command_overrides: &[
                    ("git_branch", ("orange",  "", ""), "{sep} \u{e0a0} {stdout} "),
                ],
            },
        }
    }
}

/// 12-color palette used to derive all UI colors.
/// Stored in HudConfig for runtime color resolution (accent, palette names).
#[derive(Clone)]
pub struct ThemePalette {
    pub fg: String,
    pub bg: String,
    pub dim: String,
    pub surface: String,
    pub surface_bright: String,
    pub red: String,
    pub green: String,
    pub yellow: String,
    pub blue: String,
    pub magenta: String,
    pub cyan: String,
    pub orange: String,
}

impl ThemePalette {
    /// Build a palette from zellij's `Styling` (system theme).
    ///
    /// Maps `StyleDeclaration` fields back to semantic palette colors using the
    /// known `Palette → Styling` conversion in zellij (data.rs:1577).
    pub fn from_styling(s: &Styling) -> Self {
        let fg = s.text_unselected.base;
        // Derive dim from fg. table_title.background maps to palette.gray, but
        // old-style palette themes don't define gray and new-style themes may
        // assign arbitrary colors. A dimmed fg is consistently readable.
        let dim = dim_color(fg);
        let bg = s.text_unselected.background;
        Self {
            fg: palette_color_to_hex(fg),
            bg: palette_color_to_hex(bg),
            dim: palette_color_to_hex(dim),
            surface: palette_color_to_hex(lighten_color(bg, 10)),
            surface_bright: palette_color_to_hex(lighten_color(bg, 20)),
            red: palette_color_to_hex(s.exit_code_error.base),
            green: palette_color_to_hex(s.exit_code_success.base),
            yellow: palette_color_to_hex(s.exit_code_error.emphasis_0),
            blue: palette_color_to_hex(s.ribbon_unselected.emphasis_2),
            magenta: palette_color_to_hex(s.text_unselected.emphasis_3),
            cyan: palette_color_to_hex(s.text_unselected.emphasis_1),
            orange: palette_color_to_hex(s.text_unselected.emphasis_0),
        }
    }

    /// Look up a built-in theme by name. Unknown names fall back to tokyonight.
    pub fn from_name(name: &str) -> Self {
        match name {
            "catppuccin-mocha" => Self {
                fg: "#cdd6f4".into(),
                bg: "#1e1e2e".into(),
                dim: "#585b70".into(),
                surface: "#313244".into(),
                surface_bright: "#45475a".into(),
                red: "#f38ba8".into(),
                green: "#a6e3a1".into(),
                yellow: "#f9e2af".into(),
                blue: "#89b4fa".into(),
                magenta: "#cba6f7".into(),
                cyan: "#89dceb".into(),
                orange: "#fab387".into(),
            },
            "nord" => Self {
                fg: "#eceff4".into(),
                bg: "#2e3440".into(),
                dim: "#4c566a".into(),
                surface: "#3b4252".into(),
                surface_bright: "#434c5e".into(),
                red: "#bf616a".into(),
                green: "#a3be8c".into(),
                yellow: "#ebcb8b".into(),
                blue: "#81a1c1".into(),
                magenta: "#b48ead".into(),
                cyan: "#88c0d0".into(),
                orange: "#d08770".into(),
            },
            "gruvbox-dark" => Self {
                fg: "#ebdbb2".into(),
                bg: "#282828".into(),
                dim: "#665c54".into(),
                surface: "#3c3836".into(),
                surface_bright: "#504945".into(),
                red: "#fb4934".into(),
                green: "#b8bb26".into(),
                yellow: "#fabd2f".into(),
                blue: "#83a598".into(),
                magenta: "#d3869b".into(),
                cyan: "#8ec07c".into(),
                orange: "#fe8019".into(),
            },
            // tokyonight (default)
            _ => Self::default(),
        }
    }

    /// Apply `palette_*` overrides from user config.
    pub fn apply_overrides(&mut self, config: &BTreeMap<String, String>) {
        macro_rules! override_field {
            ($key:expr, $field:expr) => {
                if let Some(v) = config.get($key) {
                    $field = v.clone();
                }
            };
        }
        override_field!("palette_fg", self.fg);
        override_field!("palette_bg", self.bg);
        override_field!("palette_dim", self.dim);
        override_field!("palette_surface", self.surface);
        override_field!("palette_surface_bright", self.surface_bright);
        override_field!("palette_red", self.red);
        override_field!("palette_green", self.green);
        override_field!("palette_yellow", self.yellow);
        override_field!("palette_blue", self.blue);
        override_field!("palette_magenta", self.magenta);
        override_field!("palette_cyan", self.cyan);
        override_field!("palette_orange", self.orange);
    }
}

impl ThemePalette {
    /// Resolve a palette color name to its hex value.
    pub fn resolve(&self, name: &str) -> Option<&str> {
        match name {
            "fg" => Some(&self.fg),
            "bg" => Some(&self.bg),
            "dim" => Some(&self.dim),
            "surface" => Some(&self.surface),
            "surface_bright" => Some(&self.surface_bright),
            "red" => Some(&self.red),
            "green" => Some(&self.green),
            "yellow" => Some(&self.yellow),
            "blue" => Some(&self.blue),
            "magenta" => Some(&self.magenta),
            "cyan" => Some(&self.cyan),
            "orange" => Some(&self.orange),
            _ => None,
        }
    }
}

/// Produce a dimmed variant of a color (50% brightness).
fn dim_color(color: PaletteColor) -> PaletteColor {
    match color {
        PaletteColor::Rgb((r, g, b)) => PaletteColor::Rgb((r / 2, g / 2, b / 2)),
        // EightBit(8) = "bright black" = dark gray in most terminals
        PaletteColor::EightBit(_) => PaletteColor::EightBit(8),
    }
}

/// Lighten a color by adding `amount` to each RGB channel (clamped to 255).
fn lighten_color(color: PaletteColor, amount: u8) -> PaletteColor {
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
fn palette_color_to_hex(color: PaletteColor) -> String {
    match color {
        PaletteColor::Rgb((r, g, b)) => format!("#{:02x}{:02x}{:02x}", r, g, b),
        PaletteColor::EightBit(n) => format!("8bit:{}", n),
    }
}

impl Default for ThemePalette {
    fn default() -> Self {
        Self {
            fg: "#c0caf5".into(),
            bg: "#1a1b26".into(),
            dim: "#565f89".into(),
            surface: "#24283b".into(),
            surface_bright: "#292e42".into(),
            red: "#f7768e".into(),
            green: "#9ece6a".into(),
            yellow: "#e0af68".into(),
            blue: "#7aa2f7".into(),
            magenta: "#bb9af7".into(),
            cyan: "#2ac3de".into(),
            orange: "#ff9e64".into(),
        }
    }
}

/// ANSI color escapes for icon categories in the tooltip.
pub struct IconColors {
    pub navigation: Color,
    pub create: Color,
    pub close: Color,
    pub resize: Color,
    pub toggle: Color,
    pub search: Color,
    pub mode_switch: Color,
    pub plugin: Color,
    pub dim: Color,
}

impl IconColors {
    fn from_palette(p: &ThemePalette) -> Self {
        let c = |hex: &str| Color::from_hex(hex).unwrap_or_default();
        Self {
            navigation: c(&p.cyan),
            create: c(&p.green),
            close: c(&p.red),
            resize: c(&p.orange),
            toggle: c(&p.yellow),
            search: c(&p.magenta),
            mode_switch: c(&p.blue),
            plugin: c(&p.fg),
            dim: c(&p.dim),
        }
    }
}

/// User-defined command widget: runs a shell command at an interval.
#[derive(Clone)]
pub struct CommandWidget {
    /// Shell command to execute.
    pub command: String,
    /// Widget style.
    pub style: WidgetStyle,
    /// Output format template. Placeholders: {stdout}, {exit_code}.
    pub format: String,
    /// Execution interval in seconds. 0 = run once.
    pub interval: u32,
}

/// User-defined static text widget.
#[derive(Clone)]
pub struct TextWidget {
    /// Static content to display.
    pub content: String,
    /// Widget style.
    pub style: WidgetStyle,
    /// Format template. Placeholder: {content}. Default: "{content}".
    pub format: String,
}

pub struct HudConfig {
    pub format_left: String,
    pub format_center: String,
    pub format_right: String,
    /// HUD bar background color (palette name or hex, resolved at render time).
    pub bar_bg: String,
    pub icon_colors: IconColors,
    // --- Tooltip settings ---
    /// Key text color (palette name or hex).
    pub tooltip_key_color: String,
    /// Separator color between key and description.
    pub tooltip_separator_color: String,
    /// Description text color.
    pub tooltip_description_color: String,
    /// Mode-switch description color.
    pub tooltip_mode_color: String,
    /// Tooltip content background (empty = default frame bg).
    pub tooltip_bg: String,
    /// Frame border color.
    pub tooltip_border_color: String,
    /// Separator character between key and description.
    pub tooltip_separator: String,
    /// Position: "bottom-right", "bottom-left", "top-right", "top-left".
    pub tooltip_position: String,
    /// Frame title template. {mode} = current mode name. Empty = no title.
    pub tooltip_title: String,
    /// Whether to show the tooltip border.
    pub tooltip_border: bool,
    pub enable_status_bar: bool,
    pub enable_tooltip: bool,
    /// Whether to use zellij's theme colors (theme "system").
    pub use_system_theme: bool,
    /// Per-mode accent color (palette name or hex). Widgets using "accent"
    /// resolve to this map at render time based on the current mode.
    pub mode_accent: HashMap<InputMode, String>,

    // --- v3 widget styles ---

    /// Mode widget style.
    pub mode_style: WidgetStyle,
    /// Mode format template. Placeholder: {content} (resolved mode text).
    pub mode_format: String,
    /// Per-mode display content (e.g., "󰍀 NORMAL").
    pub mode_content: HashMap<InputMode, String>,

    /// Session widget style.
    pub session_style: WidgetStyle,
    /// Session format template. Placeholder: {name}.
    pub session_format: String,

    /// Active tab style.
    pub tab_active_style: WidgetStyle,
    /// Inactive tab style.
    pub tab_inactive_style: WidgetStyle,
    /// Active tab format template. Placeholders: {name}, {index}, {sync_indicator}, {fullscreen_indicator}.
    pub tab_active_format: String,
    /// Inactive tab format template. Same placeholders as active.
    pub tab_inactive_format: String,
    /// Sync indicator text (shown conditionally).
    pub tab_sync_indicator: String,
    /// Fullscreen indicator text (shown conditionally).
    pub tab_fullscreen_indicator: String,
    /// Separator text inserted between adjacent tabs. Default: empty string.
    pub tab_separator_content: String,
    /// Tab separator style.
    pub tab_separator_style: WidgetStyle,
    /// Optional per-placeholder styles within tab formats.
    /// When set, the placeholder text uses this style instead of the tab style.
    pub tab_active_index_style: Option<WidgetStyle>,
    pub tab_active_name_style: Option<WidgetStyle>,
    pub tab_active_sync_style: Option<WidgetStyle>,
    pub tab_active_fullscreen_style: Option<WidgetStyle>,
    pub tab_inactive_index_style: Option<WidgetStyle>,
    pub tab_inactive_name_style: Option<WidgetStyle>,
    pub tab_inactive_sync_style: Option<WidgetStyle>,
    pub tab_inactive_fullscreen_style: Option<WidgetStyle>,
    /// Format templates for tab sub-placeholders. {content} is the value.
    pub tab_active_index_format: String,
    pub tab_active_name_format: String,
    pub tab_active_sync_format: String,
    pub tab_active_fullscreen_format: String,
    pub tab_inactive_index_format: String,
    pub tab_inactive_name_format: String,
    pub tab_inactive_sync_format: String,
    pub tab_inactive_fullscreen_format: String,

    /// CWD widget style.
    pub cwd_style: WidgetStyle,
    /// CWD format template. Placeholder: {cwd}.
    pub cwd_format: String,

    /// User-defined command widgets, keyed by name.
    pub command_widgets: HashMap<String, CommandWidget>,
    /// User-defined text widgets, keyed by name.
    pub text_widgets: HashMap<String, TextWidget>,

    /// Theme palette for runtime color resolution (accent, palette names).
    pub palette: ThemePalette,
}

impl HudConfig {
    pub fn from_config(config: &BTreeMap<String, String>) -> Self {
        let use_system_theme = config.get("theme").map_or(true, |t| t == "system");

        // For "system" (default), use tokyonight as placeholder until ModeUpdate delivers Styling.
        let mut palette = match config.get("theme") {
            Some(name) if name != "system" => ThemePalette::from_name(name),
            _ => ThemePalette::default(),
        };
        palette.apply_overrides(config);

        let mut hud = Self::build_from_palette(&palette, config);
        hud.use_system_theme = use_system_theme;
        hud
    }

    /// Rebuild colors from zellij's system theme. Called when ModeUpdate arrives.
    pub fn apply_system_theme(
        &mut self,
        styling: &Styling,
        config: &BTreeMap<String, String>,
    ) {
        let mut palette = ThemePalette::from_styling(styling);
        palette.apply_overrides(config);
        let rebuilt = Self::build_from_palette(&palette, config);

        // Update resolved color fields; preserve non-color config.
        self.icon_colors = rebuilt.icon_colors;
        self.palette = rebuilt.palette;
        // Widget styles and tooltip colors use palette names resolved
        // at render time, so they don't need rebuilding on theme change.
    }

    fn build_from_palette(palette: &ThemePalette, config: &BTreeMap<String, String>) -> Self {
        let ws = |s: WStyle| WidgetStyle::new(s.0, s.1, s.2);
        let style_name = config.get("style").map(|s| s.as_str()).unwrap_or("simple");
        let sd = StyleDefaults::from_name(style_name);
        let icon_colors = IconColors::from_palette(palette);

        let mode_accent = HashMap::from([
            (InputMode::Normal, "blue".to_string()),
            (InputMode::Locked, "red".to_string()),
            (InputMode::Resize, "yellow".to_string()),
            (InputMode::Pane, "cyan".to_string()),
            (InputMode::Tab, "green".to_string()),
            (InputMode::Scroll, "magenta".to_string()),
            (InputMode::Search, "magenta".to_string()),
            (InputMode::EnterSearch, "magenta".to_string()),
            (InputMode::RenameTab, "yellow".to_string()),
            (InputMode::RenamePane, "yellow".to_string()),
            (InputMode::Session, "cyan".to_string()),
            (InputMode::Move, "orange".to_string()),
            (InputMode::Prompt, "green".to_string()),
            (InputMode::Tmux, "orange".to_string()),
        ]);

        let mut hud = Self {
            format_left: sd.format_left.to_string(),
            format_center: sd.format_center.to_string(),
            format_right: sd.format_right.to_string(),
            bar_bg: sd.bar_bg.to_string(),
            icon_colors,
            tooltip_key_color: "cyan".to_string(),
            tooltip_separator_color: "dim".to_string(),
            tooltip_description_color: "fg".to_string(),
            tooltip_mode_color: "accent".to_string(),
            tooltip_bg: String::new(),
            tooltip_border_color: "dim".to_string(),
            tooltip_separator: "➜".to_string(),
            tooltip_position: "bottom-right".to_string(),
            tooltip_title: "{mode}".to_string(),
            tooltip_border: true,
            enable_status_bar: true,
            enable_tooltip: true,
            use_system_theme: false,
            mode_accent,

            // v3 widget styles (defaults from style preset)
            mode_style: ws(sd.mode_style),
            mode_format: sd.mode_format.to_string(),
            mode_content: HashMap::from([
                (InputMode::Normal, "󰍀 NORMAL".to_string()),
                (InputMode::Locked, "󰌾 LOCKED".to_string()),
                (InputMode::Pane, "󰘖 PANE".to_string()),
                (InputMode::Tab, "󰓩 TAB".to_string()),
                (InputMode::Resize, "󰩨 RESIZE".to_string()),
                (InputMode::Move, "󰆾 MOVE".to_string()),
                (InputMode::Scroll, "󰠶 SCROLL".to_string()),
                (InputMode::Search, "󰍉 SEARCH".to_string()),
                (InputMode::EnterSearch, "󰍉 SEARCH".to_string()),
                (InputMode::RenameTab, "󰏫 RENAME TAB".to_string()),
                (InputMode::RenamePane, "󰏫 RENAME PANE".to_string()),
                (InputMode::Session, "󱂬 SESSION".to_string()),
                (InputMode::Prompt, "󰘥 PROMPT".to_string()),
                (InputMode::Tmux, "󰰣 TMUX".to_string()),
            ]),
            session_style: ws(sd.session_style),
            session_format: sd.session_format.to_string(),
            tab_active_style: ws(sd.tab_active_style),
            tab_inactive_style: ws(sd.tab_inactive_style),
            tab_active_format: sd.tab_active_format.to_string(),
            tab_inactive_format: sd.tab_inactive_format.to_string(),
            tab_sync_indicator: "🔗".to_string(),
            tab_fullscreen_indicator: "⛶".to_string(),
            tab_separator_content: sd.tab_separator_content.to_string(),
            tab_separator_style: ws(sd.tab_separator_style),
            tab_active_index_style: sd.tab_active_index_style.map(|s| ws(s)),
            tab_active_name_style: None,
            tab_active_sync_style: sd.tab_active_sync_style.map(|s| ws(s)),
            tab_active_fullscreen_style: sd.tab_active_fullscreen_style.map(|s| ws(s)),
            tab_inactive_index_style: sd.tab_inactive_index_style.map(|s| ws(s)),
            tab_inactive_name_style: None,
            tab_inactive_sync_style: None,
            tab_inactive_fullscreen_style: None,
            tab_active_index_format: sd.tab_active_index_format.to_string(),
            tab_active_name_format: sd.tab_active_name_format.to_string(),
            tab_active_sync_format: sd.tab_active_sync_format.to_string(),
            tab_active_fullscreen_format: sd.tab_active_fullscreen_format.to_string(),
            tab_inactive_index_format: sd.tab_inactive_index_format.to_string(),
            tab_inactive_name_format: sd.tab_inactive_name_format.to_string(),
            tab_inactive_sync_format: sd.tab_inactive_sync_format.to_string(),
            tab_inactive_fullscreen_format: sd.tab_inactive_fullscreen_format.to_string(),
            cwd_style: ws(sd.cwd_style),
            cwd_format: sd.cwd_format.to_string(),
            command_widgets: HashMap::new(),
            text_widgets: HashMap::new(),
            palette: palette.clone(),
        };

        // Apply bar_bg override
        if let Some(v) = config.get("bar_bg") {
            hud.bar_bg = v.clone();
        }
        // Tooltip config overrides
        macro_rules! string_override {
            ($key:expr, $field:expr) => {
                if let Some(v) = config.get($key) {
                    $field = v.clone();
                }
            };
        }
        string_override!("tooltip_key_color", hud.tooltip_key_color);
        string_override!("tooltip_separator_color", hud.tooltip_separator_color);
        string_override!("tooltip_description_color", hud.tooltip_description_color);
        string_override!("tooltip_mode_color", hud.tooltip_mode_color);
        string_override!("tooltip_bg", hud.tooltip_bg);
        string_override!("tooltip_border_color", hud.tooltip_border_color);
        string_override!("tooltip_separator", hud.tooltip_separator);
        string_override!("tooltip_position", hud.tooltip_position);
        string_override!("tooltip_title", hud.tooltip_title);
        if let Some(v) = config.get("tooltip_border") {
            hud.tooltip_border = v != "false";
        }

        // mode_accent_* overrides (palette name or hex)
        let accent_map = [
            ("mode_accent_normal", InputMode::Normal),
            ("mode_accent_locked", InputMode::Locked),
            ("mode_accent_pane", InputMode::Pane),
            ("mode_accent_tab", InputMode::Tab),
            ("mode_accent_resize", InputMode::Resize),
            ("mode_accent_move", InputMode::Move),
            ("mode_accent_scroll", InputMode::Scroll),
            ("mode_accent_session", InputMode::Session),
            ("mode_accent_search", InputMode::Search),
            ("mode_accent_rename_tab", InputMode::RenameTab),
            ("mode_accent_rename_pane", InputMode::RenamePane),
            ("mode_accent_enter_search", InputMode::EnterSearch),
            ("mode_accent_tmux", InputMode::Tmux),
            ("mode_accent_prompt", InputMode::Prompt),
        ];
        for (key, mode) in &accent_map {
            if let Some(v) = config.get(*key) {
                hud.mode_accent.insert(*mode, v.clone());
            }
        }

        // v3 widget style overrides
        Self::parse_widget_style(config, "mode", &mut hud.mode_style);
        Self::parse_widget_style(config, "session", &mut hud.session_style);
        Self::parse_widget_style(config, "tab_active", &mut hud.tab_active_style);
        Self::parse_widget_style(config, "tab_inactive", &mut hud.tab_inactive_style);
        // Tab sub-placeholder styles (optional, fallback to tab style).
        // Only overwrite when user config provides at least one key,
        // so style-preset defaults are preserved.
        if let Some(s) = Self::parse_optional_style(config, "tab_active_index") {
            hud.tab_active_index_style = Some(s);
        }
        if let Some(s) = Self::parse_optional_style(config, "tab_active_name") {
            hud.tab_active_name_style = Some(s);
        }
        if let Some(s) = Self::parse_optional_style(config, "tab_active_sync") {
            hud.tab_active_sync_style = Some(s);
        }
        if let Some(s) = Self::parse_optional_style(config, "tab_active_fullscreen") {
            hud.tab_active_fullscreen_style = Some(s);
        }
        if let Some(s) = Self::parse_optional_style(config, "tab_inactive_index") {
            hud.tab_inactive_index_style = Some(s);
        }
        if let Some(s) = Self::parse_optional_style(config, "tab_inactive_name") {
            hud.tab_inactive_name_style = Some(s);
        }
        if let Some(s) = Self::parse_optional_style(config, "tab_inactive_sync") {
            hud.tab_inactive_sync_style = Some(s);
        }
        if let Some(s) = Self::parse_optional_style(config, "tab_inactive_fullscreen") {
            hud.tab_inactive_fullscreen_style = Some(s);
        }
        // Tab sub-placeholder format overrides
        if let Some(v) = config.get("tab_active_index_format") {
            hud.tab_active_index_format = v.clone();
        }
        if let Some(v) = config.get("tab_active_name_format") {
            hud.tab_active_name_format = v.clone();
        }
        if let Some(v) = config.get("tab_inactive_index_format") {
            hud.tab_inactive_index_format = v.clone();
        }
        if let Some(v) = config.get("tab_inactive_name_format") {
            hud.tab_inactive_name_format = v.clone();
        }
        if let Some(v) = config.get("tab_active_sync_format") {
            hud.tab_active_sync_format = v.clone();
        }
        if let Some(v) = config.get("tab_active_fullscreen_format") {
            hud.tab_active_fullscreen_format = v.clone();
        }
        if let Some(v) = config.get("tab_inactive_sync_format") {
            hud.tab_inactive_sync_format = v.clone();
        }
        if let Some(v) = config.get("tab_inactive_fullscreen_format") {
            hud.tab_inactive_fullscreen_format = v.clone();
        }

        // v3 per-mode content overrides (new: mode_content_*, fallback: mode_*)
        let mode_content_map = [
            ("mode_content_normal", "mode_normal", InputMode::Normal),
            ("mode_content_locked", "mode_locked", InputMode::Locked),
            ("mode_content_pane", "mode_pane", InputMode::Pane),
            ("mode_content_tab", "mode_tab", InputMode::Tab),
            ("mode_content_resize", "mode_resize", InputMode::Resize),
            ("mode_content_move", "mode_move", InputMode::Move),
            ("mode_content_scroll", "mode_scroll", InputMode::Scroll),
            ("mode_content_search", "mode_search", InputMode::Search),
            ("mode_content_enter_search", "mode_enter_search", InputMode::EnterSearch),
            ("mode_content_rename_tab", "mode_rename_tab", InputMode::RenameTab),
            ("mode_content_rename_pane", "mode_rename_pane", InputMode::RenamePane),
            ("mode_content_session", "mode_session", InputMode::Session),
            ("mode_content_prompt", "mode_prompt", InputMode::Prompt),
            ("mode_content_tmux", "mode_tmux", InputMode::Tmux),
        ];
        // Apply style preset mode_content defaults first, then user config overrides
        for (suffix, content) in sd.mode_content {
            // Find the InputMode for this suffix via mode_content_map
            let key = format!("mode_content_{}", suffix);
            if let Some((_, _, mode)) = mode_content_map.iter().find(|(k, _, _)| *k == key) {
                hud.mode_content.insert(*mode, content.to_string());
            }
        }
        for (new_key, old_key, mode) in &mode_content_map {
            if let Some(v) = config.get(*new_key) {
                hud.mode_content.insert(*mode, v.clone());
            } else if let Some(v) = config.get(*old_key) {
                hud.mode_content.insert(*mode, v.clone());
            }
        }

        // v3 format overrides
        if let Some(v) = config.get("mode_format") {
            hud.mode_format = v.clone();
        }
        if let Some(v) = config.get("cwd_format") {
            hud.cwd_format = v.clone();
        }
        if let Some(v) = config.get("session_format") {
            hud.session_format = v.clone();
        }
        // tab_format sets both active and inactive as fallback
        if let Some(v) = config.get("tab_format") {
            hud.tab_active_format = v.clone();
            hud.tab_inactive_format = v.clone();
        }
        if let Some(v) = config.get("tab_active_format") {
            hud.tab_active_format = v.clone();
        }
        if let Some(v) = config.get("tab_inactive_format") {
            hud.tab_inactive_format = v.clone();
        }
        if let Some(v) = config.get("tab_separator_content") {
            hud.tab_separator_content = v.clone();
        }
        Self::parse_widget_style(config, "tab_separator", &mut hud.tab_separator_style);
        if let Some(v) = config.get("tab_sync_indicator") {
            hud.tab_sync_indicator = v.clone();
        }
        if let Some(v) = config.get("tab_fullscreen_indicator") {
            hud.tab_fullscreen_indicator = v.clone();
        }
        // v3 built-in widget style overrides
        Self::parse_widget_style(config, "cwd", &mut hud.cwd_style);

        // Discover and parse user-defined widgets
        let (user_commands, user_texts) = Self::parse_user_widgets(config);
        hud.command_widgets = user_commands;
        hud.text_widgets = user_texts;

        // Built-in command widgets with fixed defaults.
        // Style presets may override style/format via command_overrides.
        let builtin_commands: Vec<(&str, CommandWidget)> = vec![
            ("time", CommandWidget {
                command: "date +%H:%M".to_string(),
                style: WidgetStyle::new("blue", "", ""),
                format: " \u{f0954} {stdout} ".to_string(),
                interval: 1,
            }),
            ("date", CommandWidget {
                command: "date +\"%b %d\"".to_string(),
                style: WidgetStyle::new("magenta", "", ""),
                format: " \u{f00ed} {stdout} ".to_string(),
                interval: 60,
            }),
            ("memory", CommandWidget {
                command: "free | awk '/Mem:/{printf \"%.0f%%\", $3/$2*100}'".to_string(),
                style: WidgetStyle::new("green", "", ""),
                format: " \u{f035b} {stdout} ".to_string(),
                interval: 5,
            }),
            ("git_branch", CommandWidget {
                command: "git rev-parse --abbrev-ref HEAD 2>/dev/null".to_string(),
                style: WidgetStyle::new("orange", "", ""),
                format: " \u{e0a0} {stdout} ".to_string(),
                interval: 10,
            }),
        ];
        for (name, widget) in builtin_commands {
            hud.command_widgets.entry(name.to_string()).or_insert(widget);
        }
        // Apply style preset overrides for built-in command widgets
        for &(name, style_tuple, format) in sd.command_overrides {
            if let Some(w) = hud.command_widgets.get_mut(name) {
                w.style = ws(style_tuple);
                w.format = format.to_string();
            }
        }
        // Apply short-name style/format overrides (e.g., time_fg, git_branch_format)
        let widget_names: Vec<String> = hud.command_widgets.keys().cloned().collect();
        for name in &widget_names {
            let w = hud.command_widgets.get_mut(name).unwrap();
            Self::parse_widget_style(config, name, &mut w.style);
            if let Some(v) = config.get(&format!("{}_format", name)) {
                w.format = v.clone();
            }
        }

        // Default text widgets (separators) based on style preset.
        let tw = |content: &str, fg: &str, bg: &str| TextWidget {
            content: content.to_string(),
            style: WidgetStyle::new(fg, bg, ""),
            format: "{content}".to_string(),
        };
        let default_texts: Vec<(&str, TextWidget)> = match style_name {
            "powerline" => vec![
                // Left: mode(bg=accent) ▶ session(bg=surface) ▶ bar_bg
                ("s_ms", tw("\u{e0b0}", "accent",  "surface")), // mode → session
                ("s_sb", tw("\u{e0b0}", "surface", "")),        // session → bar
                // Right: cwd ▸ git ◂ memory(bg=surface) ◂ time(bg=accent)
                ("s_cg", tw("\u{e0b3}", "dim",     "")),        // cwd → git (thin)
                ("s_gm", tw("\u{e0b2}", "surface", "")),        // git → memory
                ("s_mt", tw("\u{e0b2}", "accent",  "surface")), // memory → time
                // Tab powerline separators (entry/exit arrows)
                ("ta_in",  tw("\u{e0b0}", "bg",             "surface_bright")),
                ("ta_out", tw("\u{e0b0}", "surface_bright",  "bg")),
                ("ti_in",  tw("\u{e0b0}", "bg",             "surface")),
                ("ti_out", tw("\u{e0b0}", "surface",         "bg")),
            ],
            "bubble" => vec![
                // Rounded pill edges
                ("pill_left",  tw("\u{e0b6}", "next_bg", "")),
                ("pill_right", tw("\u{e0b4}", "prev_bg", "")),
                // Bar-bg gap between tabs
                ("gap", tw(" ", "", "bg")),
                // Two-tone icon badges (icon + trailing space as padding)
                ("sess_icon", tw("\u{f018d} ", "bg", "cyan")),
                ("cwd_icon",  tw("\u{f0256} ", "bg", "cyan")),
                ("git_icon",  tw("\u{e0a0} ",  "bg", "magenta")),
                ("mem_icon",  tw("\u{f035b} ", "bg", "green")),
                ("time_icon", tw("\u{f0954} ", "bg", "blue")),
                ("date_icon", tw("\u{f00ed} ", "bg", "magenta")),
            ],
            "minimal" => vec![],
            _ => vec![
                ("sep", tw("|", "dim", "")),
            ],
        };
        for (name, widget) in default_texts {
            hud.text_widgets.entry(name.to_string()).or_insert(widget);
        }
        // Apply style overrides to default text widgets
        let text_names: Vec<String> = hud.text_widgets.keys().cloned().collect();
        for name in &text_names {
            let mut style = hud.text_widgets[name].style.clone();
            Self::parse_widget_style(config, &name, &mut style);
            hud.text_widgets.get_mut(name.as_str()).unwrap().style = style;
        }

        if let Some(v) = config.get("format_left") {
            hud.format_left = v.clone();
        }
        if let Some(v) = config.get("format_center") {
            hud.format_center = v.clone();
        }
        if let Some(v) = config.get("format_right") {
            hud.format_right = v.clone();
        }
        if let Some(v) = config.get("enable_status_bar") {
            hud.enable_status_bar = v != "false";
        }
        if let Some(v) = config.get("enable_tooltip") {
            hud.enable_tooltip = v != "false";
        }

        hud
    }

    /// Reserved config prefixes that cannot be used as user widget names.
    /// Checked by exact match: "mode" is reserved, but "mode_sep" is allowed.
    const RESERVED_PREFIXES: &'static [&'static str] = &[
        "mode", "session", "tab_active", "tab_inactive", "tabs", "cwd", "bar",
        "tooltip", "palette", "format", "enable", "theme", "style", "base_mode",
        "mode_accent", "mode_content",
    ];

    /// Check if a widget name conflicts with a reserved prefix.
    fn is_reserved_name(name: &str) -> bool {
        Self::RESERVED_PREFIXES.contains(&name)
    }

    /// Discover user-defined widgets from config keys.
    ///
    /// Detection rules:
    /// - `NAME_command` → command widget
    /// - `NAME_content` → text widget
    ///
    /// Widget names must not match any reserved prefix.
    fn parse_user_widgets(
        config: &BTreeMap<String, String>,
    ) -> (HashMap<String, CommandWidget>, HashMap<String, TextWidget>) {
        let mut commands = HashMap::new();
        let mut texts = HashMap::new();

        for key in config.keys() {
            // Try NAME_command pattern
            if let Some(name) = key.strip_suffix("_command") {
                if name.is_empty() || Self::is_reserved_name(name) {
                    continue;
                }
                if commands.contains_key(name) {
                    continue;
                }
                let command = config.get(key).cloned().unwrap_or_default();
                let mut style = WidgetStyle::default();
                Self::parse_widget_style(config, name, &mut style);
                let format = config
                    .get(&format!("{name}_format"))
                    .cloned()
                    .unwrap_or_else(|| "{stdout}".to_string());
                let interval = config
                    .get(&format!("{name}_interval"))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(10);

                commands.insert(
                    name.to_string(),
                    CommandWidget { command, style, format, interval },
                );
            }

            // Try NAME_content pattern (skip if it matches mode_content_* which is per-mode content)
            if let Some(name) = key.strip_suffix("_content") {
                if name.is_empty() || name.starts_with("mode_content") {
                    continue;
                }
                if Self::is_reserved_name(name) {
                    continue;
                }
                if texts.contains_key(name) {
                    continue;
                }
                let content = config.get(key).cloned().unwrap_or_default();
                let mut style = WidgetStyle::default();
                Self::parse_widget_style(config, name, &mut style);
                let format = config
                    .get(&format!("{name}_format"))
                    .cloned()
                    .unwrap_or_else(|| "{content}".to_string());

                texts.insert(
                    name.to_string(),
                    TextWidget { content, style, format },
                );
            }
        }

        (commands, texts)
    }

    /// Parse `{prefix}_fg`, `{prefix}_bg`, `{prefix}_attr` from config into a WidgetStyle.
    fn parse_widget_style(
        config: &BTreeMap<String, String>,
        prefix: &str,
        style: &mut WidgetStyle,
    ) {
        if let Some(v) = config.get(&format!("{}_fg", prefix)) {
            style.fg = v.clone();
        }
        if let Some(v) = config.get(&format!("{}_bg", prefix)) {
            style.bg = v.clone();
        }
        if let Some(v) = config.get(&format!("{}_attr", prefix)) {
            style.attr = v.clone();
        }
    }

    /// Parse an optional widget style: returns `Some(style)` only if at least one
    /// of `{prefix}_fg`, `{prefix}_bg`, `{prefix}_attr` is present in config.
    fn parse_optional_style(
        config: &BTreeMap<String, String>,
        prefix: &str,
    ) -> Option<WidgetStyle> {
        let fg = config.get(&format!("{}_fg", prefix));
        let bg = config.get(&format!("{}_bg", prefix));
        let attr = config.get(&format!("{}_attr", prefix));
        if fg.is_some() || bg.is_some() || attr.is_some() {
            Some(WidgetStyle {
                fg: fg.cloned().unwrap_or_default(),
                bg: bg.cloned().unwrap_or_default(),
                attr: attr.cloned().unwrap_or_default(),
            })
        } else {
            None
        }
    }

    /// Resolve a palette name or hex string into a `Color`.
    fn resolve_color(value: &str, palette: &ThemePalette) -> Option<Color> {
        let hex = palette.resolve(value).unwrap_or(value);
        Color::from_hex(hex)
    }

    /// Resolve a color value that may be "accent", a palette name, or hex.
    pub fn resolve_color_with_accent(
        &self,
        value: &str,
        palette: &ThemePalette,
        mode: InputMode,
    ) -> Color {
        if value == "accent" {
            let accent_name = self
                .mode_accent
                .get(&mode)
                .map(|s| s.as_str())
                .unwrap_or("blue");
            let hex = palette.resolve(accent_name).unwrap_or(accent_name);
            Color::from_hex(hex).unwrap_or_default()
        } else {
            Self::resolve_color(value, palette).unwrap_or_default()
        }
    }
}

impl Default for HudConfig {
    fn default() -> Self {
        Self::from_config(&BTreeMap::new())
    }
}
