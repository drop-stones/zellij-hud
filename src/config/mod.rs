//! Plugin configuration: the `HudConfig` struct, KDL parsing, and theme/style
//! preset resolution.
//!
//! The parsing path goes through `HudConfig::from_config`, which seeds each
//! field from a `StyleDefaults` preset (built-in `simple`/`minimal`/
//! `powerline`/`bubble`/`custom`) and then layers user overrides on top.
//! Submodules:
//!
//! - [`color`] — `Color` enum + `PaletteColor` helpers
//! - [`style`] — `WidgetStyle`, the (private) `StyleDefaults` presets, and
//!   the user-defined widget value types
//! - [`theme`] — `ThemePalette` (12-color palette) + presets

use std::collections::{BTreeMap, HashMap};

use zellij_tile::prelude::{InputMode, Styling};

mod color;
mod style;
mod theme;

pub use color::Color;
pub use style::{CommandWidget, IconColors, TextWidget, WidgetStyle};
pub use theme::ThemePalette;

use style::{StyleDefaults, WStyle};


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
