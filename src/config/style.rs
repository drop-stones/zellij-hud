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

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::theme::ThemePalette;

    // ---- WidgetStyle ----

    #[test]
    fn widget_style_new_stores_each_field() {
        let s = WidgetStyle::new("blue", "surface", "bold");
        assert_eq!(s.fg, "blue");
        assert_eq!(s.bg, "surface");
        assert_eq!(s.attr, "bold");
    }

    #[test]
    fn widget_style_default_is_all_empty_strings() {
        // Empty strings mean "no override / inherit" downstream — pin that.
        let s = WidgetStyle::default();
        assert_eq!(s.fg, "");
        assert_eq!(s.bg, "");
        assert_eq!(s.attr, "");
    }

    // ---- StyleDefaults presets ----

    const PRESETS: &[&str] = &[
        "simple",
        "minimal",
        "powerline",
        "bubble",
        "custom",
    ];

    #[test]
    fn style_defaults_each_preset_has_well_formed_layout() {
        // Every preset must yield non-panicking values for the layout fields.
        // Empty format strings are allowed (e.g., custom and minimal use them
        // for sections they don't render).
        for name in PRESETS {
            let d = StyleDefaults::from_name(name);
            // bar_bg is either "bg" or "" (minimal goes transparent).
            assert!(
                d.bar_bg == "bg" || d.bar_bg.is_empty(),
                "preset {name}: unexpected bar_bg={:?}",
                d.bar_bg,
            );
            // mode_format must contain {content}; otherwise the mode widget
            // would render as a literal template — a silent regression.
            assert!(
                d.mode_format.contains("{content}"),
                "preset {name}: mode_format missing {{content}}: {:?}",
                d.mode_format,
            );
        }
    }

    #[test]
    fn style_defaults_unknown_name_falls_through_to_simple() {
        // The match's wildcard arm is the "simple" default — pin that contract
        // so a future restructure doesn't accidentally change the fallback.
        let unknown = StyleDefaults::from_name("does-not-exist");
        let simple = StyleDefaults::from_name("simple");
        assert_eq!(unknown.format_left, simple.format_left);
        assert_eq!(unknown.format_right, simple.format_right);
        assert_eq!(unknown.bar_bg, simple.bar_bg);
    }

    #[test]
    fn style_defaults_minimal_centres_tabs() {
        // Minimal style is the only built-in to use format_center for tabs.
        // If this changes, the rendering invariant ("non-empty center =
        // absolute centring") needs a re-think.
        let d = StyleDefaults::from_name("minimal");
        assert_eq!(d.format_center, "{tabs}");
    }

    #[test]
    fn style_defaults_bubble_uses_pill_widgets_in_format() {
        // Bubble style depends on pill_left/pill_right text widgets being
        // referenced in the format strings; pin that so accidental edits
        // don't strip them.
        let d = StyleDefaults::from_name("bubble");
        assert!(d.format_left.contains("{pill_right}"));
        assert!(d.format_left.contains("{pill_left}"));
        assert!(d.format_right.contains("{pill_left}"));
        assert!(d.format_right.contains("{pill_right}"));
    }

    #[test]
    fn style_defaults_powerline_uses_arrow_text_widgets() {
        // Powerline style depends on the s_* arrow widgets and ta_/ti_
        // tab arrow widgets. Catch accidental removal.
        let d = StyleDefaults::from_name("powerline");
        assert!(d.format_left.contains("{s_ms}"));
        assert!(d.format_left.contains("{s_sb}"));
        assert!(d.tab_active_format.contains("{ta_in}"));
        assert!(d.tab_active_format.contains("{ta_out}"));
        assert!(d.tab_inactive_format.contains("{ti_in}"));
        assert!(d.tab_inactive_format.contains("{ti_out}"));
    }

    #[test]
    fn style_defaults_custom_is_blank_slate() {
        // The "custom" preset is the documented blank-slate starting point —
        // empty format strings, no command_overrides. Pin that contract.
        let d = StyleDefaults::from_name("custom");
        assert!(d.format_left.is_empty());
        assert!(d.format_center.is_empty());
        assert!(d.format_right.is_empty());
        assert!(d.command_overrides.is_empty());
        assert!(d.mode_content.is_empty());
    }

    // ---- IconColors ----

    #[test]
    fn icon_colors_from_palette_resolves_each_category() {
        let p = ThemePalette::default();
        let icons = IconColors::from_palette(&p);
        // Tokyonight default has hex colors so each field should be Rgb.
        for (label, color) in [
            ("navigation", &icons.navigation),
            ("create", &icons.create),
            ("close", &icons.close),
            ("resize", &icons.resize),
            ("toggle", &icons.toggle),
            ("search", &icons.search),
            ("mode_switch", &icons.mode_switch),
            ("plugin", &icons.plugin),
            ("dim", &icons.dim),
        ] {
            assert!(
                matches!(color, Color::Rgb(_, _, _)),
                "{label} did not resolve to Rgb",
            );
        }
    }

    #[test]
    fn icon_colors_from_palette_falls_back_to_default_on_unparseable() {
        // Any field that can't be parsed by Color::from_hex falls back to
        // Color::None — pin that so a typo in a user palette override
        // produces a visible blank, not a panic.
        let mut p = ThemePalette::default();
        p.cyan = "not a color".into();
        let icons = IconColors::from_palette(&p);
        assert!(matches!(icons.navigation, Color::None));
        // Unaffected fields still resolve.
        assert!(matches!(icons.create, Color::Rgb(_, _, _)));
    }
}
