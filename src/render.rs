use unicode_width::UnicodeWidthChar;

use crate::config::WidgetStyle;
use crate::State;

/// Count visible display width of a string, ignoring ANSI escape sequences
/// and accounting for wide characters (CJK, nerd font icons, emoji).
pub(crate) fn visible_len(s: &str) -> usize {
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

/// Build ANSI escape for text attributes ("bold", "italic", "bold,italic").
fn attr_escape(attr: &str) -> String {
    if attr.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for part in attr.split(',') {
        match part.trim() {
            "bold" => out.push_str("\x1b[1m"),
            "italic" => out.push_str("\x1b[3m"),
            _ => {}
        }
    }
    out
}

impl State {
    /// Resolve a WidgetStyle's fg/bg/attr into ANSI escapes for the current mode.
    fn style_escapes(&self, style: &WidgetStyle) -> (String, String, String) {
        let c = &self.hud_config;
        let fg = if style.fg.is_empty() {
            String::new()
        } else {
            c.resolve_color_with_accent(&style.fg, &c.palette, self.mode).fg()
        };
        let bg = if style.bg.is_empty() {
            String::new()
        } else {
            c.resolve_color_with_accent(&style.bg, &c.palette, self.mode).bg()
        };
        let attr = attr_escape(&style.attr);
        (fg, bg, attr)
    }

    /// Resolve a separator sentinel ("thick"/"thin") or literal char,
    /// using the SeparatorPreset and direction (is_right).
    fn resolve_separator_char<'a>(&self, sep: &'a str, is_right: bool) -> &'a str {
        let preset = self.hud_config.separator;
        match sep {
            "thick" => {
                if is_right { preset.powerline_right() } else { preset.powerline_left() }
            }
            "thin" => {
                if is_right { preset.minimal_right() } else { preset.minimal_left() }
            }
            other => other,
        }
    }

    /// Render a single segment placeholder into styled text.
    pub(crate) fn render_segment(&self, placeholder: &str) -> String {
        let c = &self.hud_config;
        let reset = "\x1b[0m";

        match placeholder {
            "{mode}" => {
                let (fg, bg, attr) = self.style_escapes(&c.mode_style);
                let mode_text = c.mode_content
                    .get(&self.mode)
                    .cloned()
                    .unwrap_or_else(|| format!("{:?}", self.mode).to_uppercase());
                let content = c.mode_format.replace("{content}", &mode_text);
                format!("{bg}{fg}{attr}{content}{reset}")
            }
            "{session}" => {
                let (fg, bg, attr) = self.style_escapes(&c.session_style);
                let content = c.session_format.replace("{name}", &self.session_name);
                format!("{bg}{fg}{attr}{content}{reset}")
            }
            "{tabs}" => {
                self.render_tabs()
            }
            "{cwd}" => {
                let (fg, bg, attr) = self.style_escapes(&c.cwd_style);
                let content = c.cwd_format.replace("{cwd}", &self.format_cwd());
                format!("{bg}{fg}{attr}{content}{reset}")
            }
            _ => {
                // Aliases: {time} → {command_time}, {date} → {command_date}, etc.
                let name = placeholder.strip_prefix('{').and_then(|s| s.strip_suffix('}'));
                if let Some(name) = name {
                    if let Some(cmd_name) = name.strip_prefix("command_") {
                        self.render_command_widget(cmd_name)
                    } else if let Some(txt_name) = name.strip_prefix("text_") {
                        self.render_text_widget(txt_name)
                    } else if self.hud_config.command_widgets.contains_key(name) {
                        // {time} → command widget "time", {memory} → command widget "memory", etc.
                        self.render_command_widget(name)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            }
        }
    }

    /// Render tabs widget with internal separators.
    fn render_tabs(&self) -> String {
        let c = &self.hud_config;
        let reset = "\x1b[0m";
        let tabs_have_bg = !c.tab_active_style.bg.is_empty() || !c.tab_inactive_style.bg.is_empty();
        let mut out = String::new();

        for (i, tab) in self.tabs.iter().enumerate() {
            let style = if tab.active {
                &c.tab_active_style
            } else {
                &c.tab_inactive_style
            };
            let (fg, bg, attr) = self.style_escapes(style);
            let mut content = c.tab_format
                .replace("{name}", &tab.name)
                .replace("{index}", &(i + 1).to_string());
            if tab.is_sync_panes_active {
                content = content.replace("{sync_indicator}", &c.tab_sync_indicator);
            } else {
                content = content.replace("{sync_indicator}", "");
            }
            if tab.is_fullscreen_active {
                content = content.replace("{fullscreen_indicator}", &c.tab_fullscreen_indicator);
            } else {
                content = content.replace("{fullscreen_indicator}", "");
            }

            if tabs_have_bg {
                // Powerline-style tabs: triangle separators between tabs
                let bar_bg_c = c.resolve_color_with_accent(&c.bar_bg, &c.palette, self.mode);
                let thick = c.separator.powerline_left();

                if i == 0 {
                    // Triangle space before first tab
                    let cur_bg_name = if tab.active {
                        &c.tab_active_style.bg
                    } else {
                        &c.tab_inactive_style.bg
                    };
                    let cur_bg_name = if cur_bg_name.is_empty() { &c.bar_bg } else { cur_bg_name };
                    let cur_bg_resolved = c.resolve_color_with_accent(cur_bg_name, &c.palette, self.mode);
                    out.push_str(&format!(
                        "{}{}{thick}{reset}",
                        cur_bg_resolved.bg(), bar_bg_c.fg(),
                    ));
                } else {
                    let cur_bg_name = if tab.active {
                        &c.tab_active_style.bg
                    } else {
                        &c.tab_inactive_style.bg
                    };
                    let cur_bg_name = if cur_bg_name.is_empty() { &c.bar_bg } else { cur_bg_name };
                    let prev_active = self.tabs[i - 1].active;
                    let prev_style = if prev_active { &c.tab_active_style } else { &c.tab_inactive_style };
                    let prev_bg_name = if prev_style.bg.is_empty() { &c.bar_bg } else { &prev_style.bg };

                    let prev_bg_resolved = c.resolve_color_with_accent(prev_bg_name, &c.palette, self.mode);
                    let cur_bg_resolved = c.resolve_color_with_accent(cur_bg_name, &c.palette, self.mode);

                    // Two thick separators with bar_bg gap between tabs:
                    // prev_bg ▶ bar_bg ▶ cur_bg
                    out.push_str(&format!(
                        "{}{}{thick}{}{}{thick}{reset}",
                        bar_bg_c.bg(), prev_bg_resolved.fg(),
                        cur_bg_resolved.bg(), bar_bg_c.fg(),
                    ));
                }

                out.push_str(&format!("{bg}{fg}{attr}{content}{reset}"));
            } else {
                // Flat tabs: simple divider between tabs
                if i > 0 {
                    out.push_str(&c.tab_divider);
                }
                out.push_str(&format!("{bg}{fg}{attr}{content}{reset}"));
            }
        }

        // Closing separator after last tab (only if tab bg differs from bar)
        if tabs_have_bg {
            let last_bg_name = self.tabs.last().map(|t| {
                if t.active { &c.tab_active_style.bg } else { &c.tab_inactive_style.bg }
            }).unwrap_or(&c.bar_bg);
            let last_bg_name = if last_bg_name.is_empty() { &c.bar_bg } else { last_bg_name };
            if last_bg_name != &c.bar_bg {
                let bar_bg_resolved = c.resolve_color_with_accent(&c.bar_bg, &c.palette, self.mode);
                let last_bg_resolved = c.resolve_color_with_accent(last_bg_name, &c.palette, self.mode);
                let thick = c.separator.powerline_left();
                out.push_str(&format!("{}{}{thick}{reset}",
                    bar_bg_resolved.bg(), last_bg_resolved.fg()));
            }
        }

        out
    }

    /// Render a user-defined command widget.
    fn render_command_widget(&self, name: &str) -> String {
        let widget = match self.hud_config.command_widgets.get(name) {
            Some(w) => w,
            None => return String::new(),
        };
        let output = match self.command_outputs.get(name) {
            Some(o) => o,
            None => return String::new(),
        };

        // Hide widget on command failure or empty output
        if output.exit_code != 0 || output.stdout.is_empty() {
            return String::new();
        }

        let stdout = output.stdout.as_str();
        let exit_code = output.exit_code;

        let content = widget.format
            .replace("{stdout}", stdout)
            .replace("{exit_code}", &exit_code.to_string());

        let reset = "\x1b[0m";
        let (fg, bg, attr) = self.style_escapes(&widget.style);
        format!("{bg}{fg}{attr}{content}{reset}")
    }

    /// Render a user-defined text widget.
    fn render_text_widget(&self, name: &str) -> String {
        let widget = match self.hud_config.text_widgets.get(name) {
            Some(w) => w,
            None => return String::new(),
        };
        let reset = "\x1b[0m";
        let (fg, bg, attr) = self.style_escapes(&widget.style);
        format!("{bg}{fg}{attr}{}{reset}", widget.content)
    }

    /// Get the WidgetStyle for a given placeholder (for separator rendering).
    fn widget_style_for(&self, placeholder: &str) -> Option<&WidgetStyle> {
        let c = &self.hud_config;
        match placeholder {
            "{mode}" => Some(&c.mode_style),
            "{session}" => Some(&c.session_style),
            "{tabs}" => None, // tabs handle separators internally
            "{cwd}" => Some(&c.cwd_style),
            _ => {
                let name = placeholder.strip_prefix('{').and_then(|s| s.strip_suffix('}'))?;
                if let Some(txt_name) = name.strip_prefix("text_") {
                    c.text_widgets.get(txt_name).map(|w| &w.style)
                } else {
                    let cmd_name = name.strip_prefix("command_").unwrap_or(name);
                    c.command_widgets.get(cmd_name).map(|w| &w.style)
                }
            }
        }
    }

    /// Unified format renderer. Extracts {placeholder} tokens, renders each segment,
    /// and draws separators using each widget's separator/separator_fg/separator_bg config.
    pub(crate) fn render_format(&self, format_str: &str, is_right: bool) -> String {
        let c = &self.hud_config;
        let reset = "\x1b[0m";

        // Extract {placeholder} tokens from format string
        let mut placeholders: Vec<String> = Vec::new();
        let mut i = 0;
        let chars: Vec<char> = format_str.chars().collect();
        while i < chars.len() {
            if chars[i] == '{' {
                if let Some(end) = chars[i..].iter().position(|&ch| ch == '}') {
                    let token: String = chars[i..=i + end].iter().collect();
                    placeholders.push(token);
                    i += end + 1;
                } else {
                    i += 1;
                }
            } else {
                i += 1;
            }
        }

        // Build (rendered_content, style_ref, is_tabs) tuples, skipping empty segments
        struct Segment {
            content: String,
            placeholder: String,
        }

        let mut segments: Vec<Segment> = Vec::new();
        for ph in &placeholders {
            let rendered = self.render_segment(ph);
            if rendered.is_empty() {
                continue;
            }
            segments.push(Segment {
                content: rendered,
                placeholder: ph.clone(),
            });
        }

        if segments.is_empty() {
            return String::new();
        }

        let mut out = String::new();

        for (idx, seg) in segments.iter().enumerate() {
            let style = self.widget_style_for(&seg.placeholder);

            if is_right {
                // Right side: separator BEFORE the segment
                if idx > 0 {
                    let prev_style = self.widget_style_for(&segments[idx - 1].placeholder);
                    if let Some(ps) = prev_style {
                        if !ps.separator.is_empty() {
                            let sep_char = self.resolve_separator_char(&ps.separator, is_right);
                            let sep_fg = if ps.separator_fg.is_empty() {
                                String::new()
                            } else {
                                c.resolve_color_with_accent(&ps.separator_fg, &c.palette, self.mode).fg()
                            };
                            let sep_bg = if ps.separator_bg.is_empty() {
                                String::new()
                            } else {
                                c.resolve_color_with_accent(&ps.separator_bg, &c.palette, self.mode).bg()
                            };
                            out.push_str(&format!("{sep_bg}{sep_fg}{sep_char}{reset}"));
                        }
                    }
                }
                out.push_str(&seg.content);
            } else {
                // Left side: segment first, then separator
                out.push_str(&seg.content);

                if let Some(st) = style {
                    if !st.separator.is_empty() {
                        let sep_char = self.resolve_separator_char(&st.separator, is_right);
                        let sep_fg = if st.separator_fg.is_empty() {
                            String::new()
                        } else {
                            c.resolve_color_with_accent(&st.separator_fg, &c.palette, self.mode).fg()
                        };
                        let sep_bg = if st.separator_bg.is_empty() {
                            String::new()
                        } else {
                            c.resolve_color_with_accent(&st.separator_bg, &c.palette, self.mode).bg()
                        };
                        out.push_str(&format!("{sep_bg}{sep_fg}{sep_char}{reset}"));
                    }
                }
            }
        }

        // Right side: handle the last segment's separator (trailing)
        if is_right {
            // No trailing separator needed for rightmost segment
        }

        out
    }

    pub(crate) fn format_cwd(&self) -> String {
        if let Some(name) = self.cwd.file_name() {
            name.to_string_lossy().to_string()
        } else {
            self.cwd.to_string_lossy().to_string()
        }
    }
}
