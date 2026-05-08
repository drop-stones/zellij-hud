//! Per-widget styles, style presets, and widget value types.
//!
//! `WidgetStyle` stores the (fg, bg, attr) triple used by every widget. The
//! private `StyleDefaults` carries the per-preset (`simple`/`minimal`/
//! `powerline`/`bubble`/`custom`) defaults that `HudConfig::from_config`
//! seeds before applying user overrides. `IconColors`, `CommandWidget`, and
//! `TextWidget` are config-side value types referenced by `HudConfig`.

use super::color::Color;
use super::theme::ThemePalette;

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
pub(super) type WStyle = (&'static str, &'static str, &'static str);

/// Style preset for the status bar and tooltip.
/// Provides default widget styles; user overrides apply on top.
pub(super) struct StyleDefaults {
    pub(super) format_left: &'static str,
    pub(super) format_center: &'static str,
    pub(super) format_right: &'static str,
    pub(super) bar_bg: &'static str,
    pub(super) mode_format: &'static str,
    pub(super) mode_style: WStyle,
    /// Per-mode content overrides: (mode_suffix, content_text).
    /// mode_suffix corresponds to the suffix in `mode_content_{suffix}` config keys.
    /// Empty slice = use global defaults (icons + uppercase names).
    pub(super) mode_content: &'static [(&'static str, &'static str)],
    pub(super) session_format: &'static str,
    pub(super) session_style: WStyle,
    pub(super) tab_active_format: &'static str,
    pub(super) tab_inactive_format: &'static str,
    pub(super) tab_separator_content: &'static str,
    pub(super) tab_separator_style: WStyle,
    pub(super) tab_active_style: WStyle,
    pub(super) tab_inactive_style: WStyle,
    pub(super) tab_active_index_style: Option<WStyle>,
    pub(super) tab_active_index_format: &'static str,
    pub(super) tab_active_name_format: &'static str,
    pub(super) tab_active_sync_style: Option<WStyle>,
    pub(super) tab_active_sync_format: &'static str,
    pub(super) tab_active_fullscreen_style: Option<WStyle>,
    pub(super) tab_active_fullscreen_format: &'static str,
    pub(super) tab_inactive_index_style: Option<WStyle>,
    pub(super) tab_inactive_index_format: &'static str,
    pub(super) tab_inactive_name_format: &'static str,
    pub(super) tab_inactive_sync_format: &'static str,
    pub(super) tab_inactive_fullscreen_format: &'static str,
    pub(super) cwd_format: &'static str,
    pub(super) cwd_style: WStyle,
    /// Per-built-in-command-widget style/format overrides.
    /// These override the fixed defaults when a style preset needs a different look.
    pub(super) command_overrides: &'static [(&'static str, WStyle, &'static str)],
}

impl StyleDefaults {
    pub(super) fn from_name(name: &str) -> Self {
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
    pub(super) fn from_palette(p: &ThemePalette) -> Self {
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
