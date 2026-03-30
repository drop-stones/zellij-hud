use std::collections::{BTreeMap, HashMap};

use zellij_tile::prelude::{InputMode, PaletteColor, Styling};

/// RGB or 8-bit terminal color, used throughout the HUD for fg and bg rendering.
#[derive(Clone, Default)]
pub(crate) enum Color {
    #[default]
    None,
    Rgb(u8, u8, u8),
    EightBit(u8),
}

impl Color {
    /// ANSI foreground escape sequence.
    pub(crate) fn fg(&self) -> String {
        match self {
            Color::None => String::new(),
            Color::Rgb(r, g, b) => format!("\x1b[38;2;{};{};{}m", r, g, b),
            Color::EightBit(n) => format!("\x1b[38;5;{}m", n),
        }
    }
    /// ANSI background escape sequence.
    pub(crate) fn bg(&self) -> String {
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
pub(crate) struct WidgetStyle {
    /// Foreground color: palette name, hex, or "accent".
    pub(crate) fg: String,
    /// Background color: palette name, hex, or "accent". Empty = no bg.
    pub(crate) bg: String,
    /// Text decorations: "bold", "italic", "bold,italic", or "".
    pub(crate) attr: String,
}

impl WidgetStyle {
    pub(crate) fn new(fg: &str, bg: &str, attr: &str) -> Self {
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
    format_right: &'static str,
    bar_bg: &'static str,
    mode_format: &'static str,
    mode_style: WStyle,
    session_format: &'static str,
    session_style: WStyle,
    tab_active_format: &'static str,
    tab_inactive_format: &'static str,
    tab_active_style: WStyle,
    tab_inactive_style: WStyle,
    cwd_format: &'static str,
    cwd_style: WStyle,
    date_format: &'static str,
    date_style: WStyle,
    time_format: &'static str,
    time_style: WStyle,
    memory_format: &'static str,
    memory_style: WStyle,
    git_branch_format: &'static str,
    git_branch_style: WStyle,
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
                format_left: "{mode}{s_ms}{session}{s_sb}{tabs}",
                format_right: "{cwd}{git_branch}{s_gm}{memory}{s_mt}{time}",
                bar_bg: "bg",
                mode_format:        " {content} ",
                mode_style:         ("bg",     "accent",  "bold"),
                session_format:     " 󰆍 {name} ",
                session_style:      ("accent", "dim",     ""),
                tab_active_format:  "{ta_in} {name} {ta_out}",
                tab_inactive_format: "{ti_in} {name} {ti_out}",
                tab_active_style:   ("fg",     "#484848", "bold"),
                tab_inactive_style: ("dim",    "#282828", ""),
                cwd_format:         " \u{f0256} {cwd} ",
                cwd_style:          ("cyan",   "",        ""),
                git_branch_format:  "{s_cg} \u{e0a0} {stdout} ",
                git_branch_style:   ("orange", "",        ""),
                memory_format:      " \u{f035b} {stdout} ",
                memory_style:       ("accent", "dim",     ""),
                date_format:        " \u{f00ed} {stdout} ",
                date_style:         ("bg",     "accent",  ""),
                time_format:        " \u{f0954} {stdout} ",
                time_style:         ("bg",     "accent",  ""),
            },
            // "minimal" (default): flat look with thin separators.
            // A single "sep" text widget is defined in build_from_palette().
            // git_branch includes a leading {sep} so the separator hides
            // when the widget is empty (not in a git repo).
            _ => Self {
                format_left: "{mode}{sep}{session}{sep}{tabs}",
                format_right: "{cwd}{git_branch}{sep}{memory}{sep}{time}",
                bar_bg: "bg",
                mode_format:        " {content} ",
                mode_style:         ("accent", "",  "bold"),
                session_format:     " 󰆍 {name} ",
                session_style:      ("cyan",   "",  ""),
                tab_active_format:  " {name}",
                tab_inactive_format: " {name}",
                tab_active_style:   ("fg",     "",  "bold"),
                tab_inactive_style: ("dim",    "",  ""),
                cwd_format:         " \u{f0256} {cwd} ",
                cwd_style:          ("cyan",   "",  ""),
                git_branch_format:  "{sep} \u{e0a0} {stdout} ",
                git_branch_style:   ("orange", "",  ""),
                memory_format:      " \u{f035b} {stdout} ",
                memory_style:       ("green",  "",  ""),
                date_format:        " \u{f00ed} {stdout} ",
                date_style:         ("magenta","",  ""),
                time_format:        " \u{f0954} {stdout} ",
                time_style:         ("blue",   "",  ""),
            },
        }
    }
}

/// 10-color palette used to derive all UI colors.
/// Stored in HudConfig for runtime color resolution (accent, palette names).
#[derive(Clone)]
pub(crate) struct ThemePalette {
    pub(crate) fg: String,
    pub(crate) bg: String,
    pub(crate) dim: String,
    pub(crate) red: String,
    pub(crate) green: String,
    pub(crate) yellow: String,
    pub(crate) blue: String,
    pub(crate) magenta: String,
    pub(crate) cyan: String,
    pub(crate) orange: String,
}

impl ThemePalette {
    /// Build a palette from zellij's `Styling` (system theme).
    ///
    /// Maps `StyleDeclaration` fields back to semantic palette colors using the
    /// known `Palette → Styling` conversion in zellij (data.rs:1577).
    pub(crate) fn from_styling(s: &Styling) -> Self {
        let fg = s.text_unselected.base;
        // Derive dim from fg. table_title.background maps to palette.gray, but
        // old-style palette themes don't define gray and new-style themes may
        // assign arbitrary colors. A dimmed fg is consistently readable.
        let dim = dim_color(fg);
        Self {
            fg: palette_color_to_hex(fg),
            bg: palette_color_to_hex(s.text_unselected.background),
            dim: palette_color_to_hex(dim),
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
    pub(crate) fn from_name(name: &str) -> Self {
        match name {
            "catppuccin-mocha" => Self {
                fg: "#cdd6f4".into(),
                bg: "#1e1e2e".into(),
                dim: "#585b70".into(),
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
    pub(crate) fn apply_overrides(&mut self, config: &BTreeMap<String, String>) {
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
    pub(crate) fn resolve(&self, name: &str) -> Option<&str> {
        match name {
            "fg" => Some(&self.fg),
            "bg" => Some(&self.bg),
            "dim" => Some(&self.dim),
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
pub(crate) struct IconColors {
    pub(crate) navigation: Color,
    pub(crate) create: Color,
    pub(crate) close: Color,
    pub(crate) resize: Color,
    pub(crate) toggle: Color,
    pub(crate) search: Color,
    pub(crate) mode_switch: Color,
    pub(crate) plugin: Color,
    pub(crate) dim: Color,
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
pub(crate) struct CommandWidget {
    /// Shell command to execute.
    pub(crate) command: String,
    /// Widget style.
    pub(crate) style: WidgetStyle,
    /// Output format template. Placeholders: {stdout}, {stderr}, {exit_code}.
    pub(crate) format: String,
    /// Execution interval in seconds. 0 = run once.
    pub(crate) interval: u32,
}

/// User-defined static text widget.
#[derive(Clone)]
pub(crate) struct TextWidget {
    /// Static content to display.
    pub(crate) content: String,
    /// Widget style.
    pub(crate) style: WidgetStyle,
    /// Format template. Placeholder: {content}. Default: "{content}".
    pub(crate) format: String,
}

pub(crate) struct HudConfig {
    pub(crate) format_left: String,
    pub(crate) format_right: String,
    /// HUD bar background color (palette name or hex, resolved at render time).
    pub(crate) bar_bg: String,
    pub(crate) icon_colors: IconColors,
    // --- Tooltip settings ---
    /// Key text color (palette name or hex).
    pub(crate) tooltip_key_color: String,
    /// Separator color between key and description.
    pub(crate) tooltip_separator_color: String,
    /// Description text color.
    pub(crate) tooltip_description_color: String,
    /// Mode-switch description color.
    pub(crate) tooltip_mode_color: String,
    /// Tooltip content background (empty = default frame bg).
    pub(crate) tooltip_bg: String,
    /// Frame border color.
    pub(crate) tooltip_border_color: String,
    /// Separator character between key and description.
    pub(crate) tooltip_separator: String,
    /// Position: "bottom-right", "bottom-left", "top-right", "top-left".
    pub(crate) tooltip_position: String,
    /// Frame title template. {mode} = current mode name. Empty = no title.
    pub(crate) tooltip_title: String,
    /// Whether to show the tooltip border.
    pub(crate) tooltip_border: bool,
    pub(crate) enable_status_bar: bool,
    pub(crate) enable_tooltip: bool,
    /// Whether to use zellij's theme colors (theme "system").
    pub(crate) use_system_theme: bool,
    /// Per-mode accent color (palette name or hex). Widgets using "accent"
    /// resolve to this map at render time based on the current mode.
    pub(crate) mode_accent: HashMap<InputMode, String>,

    // --- v3 widget styles ---

    /// Mode widget style.
    pub(crate) mode_style: WidgetStyle,
    /// Mode format template. Placeholder: {content} (resolved mode text).
    pub(crate) mode_format: String,
    /// Per-mode display content (e.g., "󰍀 NORMAL").
    pub(crate) mode_content: HashMap<InputMode, String>,

    /// Session widget style.
    pub(crate) session_style: WidgetStyle,
    /// Session format template. Placeholder: {name}.
    pub(crate) session_format: String,

    /// Active tab style.
    pub(crate) tab_active_style: WidgetStyle,
    /// Inactive tab style.
    pub(crate) tab_inactive_style: WidgetStyle,
    /// Active tab format template. Placeholders: {name}, {index}, {sync_indicator}, {fullscreen_indicator}.
    pub(crate) tab_active_format: String,
    /// Inactive tab format template. Same placeholders as active.
    pub(crate) tab_inactive_format: String,
    /// Sync indicator text (shown conditionally).
    pub(crate) tab_sync_indicator: String,
    /// Fullscreen indicator text (shown conditionally).
    pub(crate) tab_fullscreen_indicator: String,

    /// CWD widget style.
    pub(crate) cwd_style: WidgetStyle,
    /// CWD format template. Placeholder: {cwd}.
    pub(crate) cwd_format: String,

    /// User-defined command widgets, keyed by name.
    pub(crate) command_widgets: HashMap<String, CommandWidget>,
    /// User-defined text widgets, keyed by name.
    pub(crate) text_widgets: HashMap<String, TextWidget>,

    /// Theme palette for runtime color resolution (accent, palette names).
    pub(crate) palette: ThemePalette,
}

impl HudConfig {
    pub(crate) fn from_config(config: &BTreeMap<String, String>) -> Self {
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
    pub(crate) fn apply_system_theme(
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
        let style_name = config.get("style").map(|s| s.as_str()).unwrap_or("minimal");
        let sd = StyleDefaults::from_name(style_name);
        let icon_colors = IconColors::from_palette(palette);

        let mode_accent = HashMap::from([
            (InputMode::Normal, "green".to_string()),
            (InputMode::Locked, "red".to_string()),
            (InputMode::Resize, "yellow".to_string()),
            (InputMode::Pane, "blue".to_string()),
            (InputMode::Tab, "blue".to_string()),
            (InputMode::Scroll, "cyan".to_string()),
            (InputMode::Search, "magenta".to_string()),
            (InputMode::EnterSearch, "magenta".to_string()),
            (InputMode::RenameTab, "yellow".to_string()),
            (InputMode::RenamePane, "yellow".to_string()),
            (InputMode::Session, "cyan".to_string()),
            (InputMode::Move, "orange".to_string()),
            (InputMode::Prompt, "cyan".to_string()),
            (InputMode::Tmux, "orange".to_string()),
        ]);

        let mut hud = Self {
            format_left: sd.format_left.to_string(),
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

        // Default command widgets (can be overridden by user config).
        // Short names work as format placeholders: {time}, {memory}, {git_branch}.
        let defaults: Vec<(&str, CommandWidget)> = vec![
            ("time", CommandWidget {
                command: "date +%H:%M".to_string(),
                style: ws(sd.time_style),
                format: sd.time_format.to_string(),
                interval: 1,
            }),
            ("date", CommandWidget {
                command: "date +\"%b %d\"".to_string(),
                style: ws(sd.date_style),
                format: sd.date_format.to_string(),
                interval: 60,
            }),
            ("memory", CommandWidget {
                command: "free | awk '/Mem:/{printf \"%.0f%%\", $3/$2*100}'".to_string(),
                style: ws(sd.memory_style),
                format: sd.memory_format.to_string(),
                interval: 5,
            }),
            ("git_branch", CommandWidget {
                command: "git rev-parse --abbrev-ref HEAD 2>/dev/null".to_string(),
                style: ws(sd.git_branch_style),
                format: sd.git_branch_format.to_string(),
                interval: 10,
            }),
        ];
        for (name, widget) in defaults {
            hud.command_widgets.entry(name.to_string()).or_insert(widget);
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
                // Left: mode(bg=accent) ▶ session(bg=dim) ▶ bar_bg
                ("s_ms", tw("\u{e0b0}", "accent", "dim")),     // mode → session
                ("s_sb", tw("\u{e0b0}", "dim",    "")),         // session → bar
                // Right: cwd ▸ git ◂ memory(bg=dim) ◂ time(bg=accent)
                ("s_cg", tw("\u{e0b3}", "dim",    "")),         // cwd → git (thin)
                ("s_gm", tw("\u{e0b2}", "dim",    "")),         // git → memory
                ("s_mt", tw("\u{e0b2}", "accent", "dim")),      // memory → time
                // Tab powerline separators (entry/exit arrows)
                ("ta_in",  tw("\u{e0b0}", "bg",      "#484848")),
                ("ta_out", tw("\u{e0b0}", "#484848", "bg")),
                ("ti_in",  tw("\u{e0b0}", "bg",      "#282828")),
                ("ti_out", tw("\u{e0b0}", "#282828", "bg")),
            ],
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
    /// - `NAME_command` → command widget (also accepts `command_NAME_command` for compat)
    /// - `NAME_content` → text widget (also accepts `text_NAME_content` for compat)
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
                if name.is_empty() {
                    continue;
                }
                // Handle command_NAME_command compat: extract inner name
                let widget_name = name.strip_prefix("command_").unwrap_or(name);
                if widget_name.is_empty() || Self::is_reserved_name(widget_name) {
                    continue;
                }
                if commands.contains_key(widget_name) {
                    continue;
                }
                let command = config.get(key).cloned().unwrap_or_default();
                let mut style = WidgetStyle::default();
                // Try new-style keys (NAME_fg) first, then old-style (command_NAME_fg)
                Self::parse_widget_style(config, widget_name, &mut style);
                if widget_name != name {
                    // Also check old-style prefixed keys as fallback
                    let mut old_style = WidgetStyle::default();
                    Self::parse_widget_style(config, name, &mut old_style);
                    if style.fg.is_empty() { style.fg = old_style.fg; }
                    if style.bg.is_empty() { style.bg = old_style.bg; }
                    if style.attr.is_empty() { style.attr = old_style.attr; }
                }
                let format = config
                    .get(&format!("{}_format", widget_name))
                    .or_else(|| config.get(&format!("{}_format", name)))
                    .cloned()
                    .unwrap_or_else(|| "{stdout}".to_string());
                let interval = config
                    .get(&format!("{}_interval", widget_name))
                    .or_else(|| config.get(&format!("{}_interval", name)))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(10);

                commands.insert(
                    widget_name.to_string(),
                    CommandWidget { command, style, format, interval },
                );
            }

            // Try NAME_content pattern (skip if it matches mode_content_* which is per-mode content)
            if let Some(name) = key.strip_suffix("_content") {
                if name.is_empty() {
                    continue;
                }
                // Skip mode_content_MODE keys (handled separately for mode widget)
                if name.starts_with("mode_content") {
                    continue;
                }
                // Handle text_NAME_content compat: extract inner name
                let widget_name = name.strip_prefix("text_").unwrap_or(name);
                if widget_name.is_empty() || Self::is_reserved_name(widget_name) {
                    continue;
                }
                if texts.contains_key(widget_name) {
                    continue;
                }
                let content = config.get(key).cloned().unwrap_or_default();
                let mut style = WidgetStyle::default();
                Self::parse_widget_style(config, widget_name, &mut style);
                if widget_name != name {
                    let mut old_style = WidgetStyle::default();
                    Self::parse_widget_style(config, name, &mut old_style);
                    if style.fg.is_empty() { style.fg = old_style.fg; }
                    if style.bg.is_empty() { style.bg = old_style.bg; }
                    if style.attr.is_empty() { style.attr = old_style.attr; }
                }
                let format = config
                    .get(&format!("{}_format", widget_name))
                    .or_else(|| config.get(&format!("{}_format", name)))
                    .cloned()
                    .unwrap_or_else(|| "{content}".to_string());

                texts.insert(
                    widget_name.to_string(),
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

    /// Resolve a palette name or hex string into a `Color`.
    fn resolve_color(value: &str, palette: &ThemePalette) -> Option<Color> {
        let hex = palette.resolve(value).unwrap_or(value);
        Color::from_hex(hex)
    }

    /// Resolve a color value that may be "accent", a palette name, or hex.
    pub(crate) fn resolve_color_with_accent(
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
