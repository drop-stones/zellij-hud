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

    /// Render a single segment placeholder into styled text.
    pub(crate) fn render_segment(&self, placeholder: &str) -> String {
        let c = &self.hud_config;
        let reset = "\x1b[0m";

        match placeholder {
            "{mode}" => {
                let (fg, bg, attr) = self.style_escapes(&c.mode_style);
                let content = c.mode_content
                    .get(&self.mode)
                    .cloned()
                    .unwrap_or_else(|| format!("{:?}", self.mode).to_uppercase());
                format!("{bg}{fg}{attr} {content} {reset}")
            }
            "{session}" => {
                let (fg, bg, attr) = self.style_escapes(&c.session_style);
                let content = c.session_format.replace("{name}", &self.session_name);
                if bg.is_empty() {
                    format!("{fg}{attr}{content}{reset}")
                } else {
                    format!("{bg}{fg}{attr} {content} {reset}")
                }
            }
            "{tabs}" => {
                let powerline = self.is_powerline();
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

                    if powerline {
                        {
                            let bar_bg_c = c.resolve_color_with_accent(&c.bar_bg, &c.palette, self.mode);
                            let thick = c.separator.powerline_left();

                            if i == 0 {
                                // Triangle space before first tab (same as inter-tab gap)
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
                        }

                        if bg.is_empty() {
                            out.push_str(&format!("{fg}{attr} {content} {reset}"));
                        } else {
                            out.push_str(&format!("{bg}{fg}{attr} {content} {reset}"));
                        }
                    } else {
                        if i > 0 {
                            out.push_str(&c.tab_divider);
                        }
                        if bg.is_empty() {
                            out.push_str(&format!("{fg}{attr}{content}{reset}"));
                        } else {
                            out.push_str(&format!("{bg}{fg}{attr} {content} {reset}"));
                        }
                    }
                }

                // Powerline: closing separator after last tab (only if tab bg differs from bar)
                if powerline {
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
            "{cwd}" => {
                let (fg, bg, attr) = self.style_escapes(&c.cwd_style);
                let content = format!("\u{f0256} {}", self.format_cwd());
                if bg.is_empty() {
                    format!("{fg}{attr}{content}{reset}")
                } else {
                    format!("{bg}{fg}{attr} {content} {reset}")
                }
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
        if bg.is_empty() {
            format!("{fg}{attr}{content}{reset}")
        } else {
            format!("{bg}{fg}{attr} {content} {reset}")
        }
    }

    /// Render a user-defined text widget.
    fn render_text_widget(&self, name: &str) -> String {
        let widget = match self.hud_config.text_widgets.get(name) {
            Some(w) => w,
            None => return String::new(),
        };
        let reset = "\x1b[0m";
        let (fg, bg, attr) = self.style_escapes(&widget.style);
        if bg.is_empty() {
            format!("{fg}{attr}{}{reset}", widget.content)
        } else {
            format!("{bg}{fg}{attr} {} {reset}", widget.content)
        }
    }

    /// Get the resolved bg color string for a segment (empty if no bg).
    fn segment_bg(&self, placeholder: &str) -> String {
        let c = &self.hud_config;
        let style = match placeholder {
            "{mode}" => &c.mode_style,
            "{session}" => &c.session_style,
            "{tabs}" => return String::new(), // tabs have per-tab bg
            "{cwd}" => &c.cwd_style,
            _ => {
                let name = placeholder.strip_prefix('{').and_then(|s| s.strip_suffix('}'));
                if let Some(name) = name {
                    if let Some(txt_name) = name.strip_prefix("text_") {
                        if let Some(w) = c.text_widgets.get(txt_name) {
                            &w.style
                        } else {
                            return String::new();
                        }
                    } else {
                        let cmd_name = name.strip_prefix("command_").unwrap_or(name);
                        if let Some(w) = c.command_widgets.get(cmd_name) {
                            &w.style
                        } else {
                            return String::new();
                        }
                    }
                } else {
                    return String::new();
                }
            }
        };
        if style.bg.is_empty() {
            String::new()
        } else {
            style.bg.clone()
        }
    }

    /// Get the bg color name of the first tab (tabs area = bar_bg).
    fn first_tab_bg_name(&self) -> &str {
        &self.hud_config.bar_bg
    }

    /// Get the bg color name of the last tab (tabs area = bar_bg).
    fn last_tab_bg_name(&self) -> &str {
        &self.hud_config.bar_bg
    }

    /// Check if this is powerline mode (any widget has a bg set).
    fn is_powerline(&self) -> bool {
        let c = &self.hud_config;
        !c.mode_style.bg.is_empty() || !c.session_style.bg.is_empty()
    }

    pub(crate) fn render_format(&self, format_str: &str, is_right: bool) -> String {
        if self.is_powerline() {
            self.render_format_powerline(format_str, is_right)
        } else {
            self.render_format_minimal(format_str, is_right)
        }
    }

    fn render_format_minimal(&self, format_str: &str, is_right: bool) -> String {
        let c = &self.hud_config;
        let reset = "\x1b[0m";
        let sep_char = if is_right {
            c.separator.minimal_right()
        } else {
            c.separator.minimal_left()
        };
        let sep_color = c.resolve_color_with_accent(&c.separator_color, &c.palette, self.mode);
        let sep = format!("{}{}{reset}", sep_color.fg(), sep_char);

        let parts: Vec<&str> = format_str.split(" | ").collect();
        let mut out = String::new();

        for part in &parts {
            let trimmed = part.trim();
            let rendered = self.render_segment(trimmed);
            if rendered.is_empty() {
                continue;
            }
            if !out.is_empty() {
                out.push_str(&format!(" {sep} "));
            }
            out.push_str(&rendered);
        }

        out
    }

    fn render_format_powerline(&self, format_str: &str, is_right: bool) -> String {
        let c = &self.hud_config;
        let reset = "\x1b[0m";
        let sep_char = if is_right {
            c.separator.powerline_right()
        } else {
            c.separator.powerline_left()
        };

        // Extract {placeholder} tokens from format string
        let mut placeholders: Vec<String> = Vec::new();
        let mut i = 0;
        let chars: Vec<char> = format_str.chars().collect();
        while i < chars.len() {
            if chars[i] == '{' {
                if let Some(end) = chars[i..].iter().position(|&c| c == '}') {
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

        struct Segment {
            content: String,
            bg_name: String,
            is_tabs: bool,
        }

        let mut segments: Vec<Segment> = Vec::new();
        for ph in &placeholders {
            let rendered = self.render_segment(ph);
            if rendered.is_empty() {
                continue;
            }
            let bg_name = self.segment_bg(ph);
            segments.push(Segment {
                content: rendered,
                bg_name,
                is_tabs: ph == "{tabs}",
            });
        }

        if segments.is_empty() {
            return String::new();
        }

        let bar_bg = &c.bar_bg;
        let mut out = String::new();

        for (i, seg) in segments.iter().enumerate() {
            // {tabs} handles its own powerline separators internally
            if seg.is_tabs {
                out.push_str(&seg.content);
                continue;
            }

            let prev_bg = if i == 0 {
                bar_bg
            } else {
                let prev = &segments[i - 1];
                if prev.is_tabs {
                    self.last_tab_bg_name()
                } else if prev.bg_name.is_empty() { bar_bg } else { &prev.bg_name }
            };
            let cur_bg = if seg.bg_name.is_empty() { bar_bg } else { &seg.bg_name };

            // No-bg segments need space padding (bg segments pad internally)
            let needs_padding = seg.bg_name.is_empty();

            if is_right {
                // Right side: separator comes BEFORE the segment
                if prev_bg == cur_bg {
                    if i > 0 {
                        // Same bg layer: thin separator with dim color
                        let thin = c.separator.minimal_right();
                        let dim = c.resolve_color_with_accent("dim", &c.palette, self.mode).fg();
                        let bg_esc = c.resolve_color_with_accent(cur_bg, &c.palette, self.mode).bg();
                        out.push_str(&format!("{bg_esc}{dim}{thin}{reset}"));
                    }
                } else {
                    // Different bg: thick powerline arrow
                    let sep_fg = c.resolve_color_with_accent(cur_bg, &c.palette, self.mode).fg();
                    let sep_bg = c.resolve_color_with_accent(prev_bg, &c.palette, self.mode).bg();
                    out.push_str(&format!("{sep_bg}{sep_fg}{sep_char}{reset}"));
                }
                if needs_padding { out.push(' '); }
                out.push_str(&seg.content);
                if needs_padding { out.push(' '); }
            } else {
                // Left side: segment first, then separator
                if needs_padding { out.push(' '); }
                out.push_str(&seg.content);
                if needs_padding { out.push(' '); }

                let next_bg = if i + 1 < segments.len() {
                    let next = &segments[i + 1];
                    if next.is_tabs {
                        self.first_tab_bg_name()
                    } else if next.bg_name.is_empty() { bar_bg } else { &next.bg_name }
                } else {
                    bar_bg
                };
                if cur_bg == next_bg {
                    // Same bg layer: thin separator with dim color
                    let thin = c.separator.minimal_left();
                    let dim = c.resolve_color_with_accent("dim", &c.palette, self.mode).fg();
                    let bg_esc = c.resolve_color_with_accent(cur_bg, &c.palette, self.mode).bg();
                    out.push_str(&format!("{bg_esc}{dim}{thin}{reset}"));
                } else {
                    // Different bg: thick powerline arrow
                    let sep_fg = c.resolve_color_with_accent(cur_bg, &c.palette, self.mode).fg();
                    let sep_bg = c.resolve_color_with_accent(next_bg, &c.palette, self.mode).bg();
                    out.push_str(&format!("{sep_bg}{sep_fg}{sep_char}{reset}"));
                }
            }
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
