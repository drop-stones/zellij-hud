use unicode_width::UnicodeWidthChar;

use crate::config::{Color, WidgetStyle};
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
                format!("{bg}{fg}{attr}{content}{reset}")
            }
            "{tabs}" => {
                let mut out = String::new();
                for (i, tab) in self.tabs.iter().enumerate() {
                    if i > 0 {
                        out.push_str(&c.tab_divider);
                    }
                    let style = if tab.active {
                        &c.tab_active_style
                    } else {
                        &c.tab_inactive_style
                    };
                    let (fg, bg, attr) = self.style_escapes(style);
                    let mut content = c.tab_format
                        .replace("{name}", &tab.name)
                        .replace("{index}", &(i + 1).to_string());
                    // Conditional indicators
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
                    if bg.is_empty() {
                        out.push_str(&format!("{fg}{attr}{content}{reset}"));
                    } else {
                        out.push_str(&format!("{bg}{fg}{attr} {content} {reset}"));
                    }
                }
                out
            }
            // Legacy built-in segments (will become command widgets in the future)
            "{cwd}" => {
                format!("{}󰉖 {}{reset}", c.color_cwd.fg(), self.format_cwd())
            }
            "{date}" => {
                format!("{}󰃭 {}{reset}", c.color_date.fg(), self.format_date())
            }
            "{time}" => {
                format!("{}󰥔 {}{reset}", c.color_time.fg(), self.format_time())
            }
            "{memory}" => {
                if self.memory_text.is_empty() {
                    String::new()
                } else {
                    format!("{}󰍛 {}{reset}", c.color_memory.fg(), self.memory_text)
                }
            }
            _ => {
                // Try command_NAME or text_NAME
                if let Some(name) = placeholder.strip_prefix("{command_").and_then(|s| s.strip_suffix('}')) {
                    self.render_command_widget(name)
                } else if let Some(name) = placeholder.strip_prefix("{text_").and_then(|s| s.strip_suffix('}')) {
                    self.render_text_widget(name)
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
        let output = self.command_outputs.get(name);
        let stdout = output.map(|o| o.stdout.as_str()).unwrap_or("");
        let exit_code = output.map(|o| o.exit_code).unwrap_or(0);

        if stdout.is_empty() && widget.format == "{stdout}" {
            return String::new();
        }

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

    /// The segment's representative color. Used as bg in powerline mode.
    fn segment_color(&self, placeholder: &str) -> Color {
        let c = &self.hud_config;
        match placeholder {
            "{mode}" => {
                if c.mode_style.bg.is_empty() {
                    c.color_for_mode(self.mode).clone()
                } else {
                    c.resolve_color_with_accent(&c.mode_style.bg, &c.palette, self.mode)
                }
            }
            "{session}" => {
                if c.session_style.fg.is_empty() {
                    c.color_session.clone()
                } else {
                    c.resolve_color_with_accent(&c.session_style.fg, &c.palette, self.mode)
                }
            }
            "{tabs}" => c.color_tab_active.clone(),
            "{cwd}" => c.color_cwd.clone(),
            "{date}" => c.color_date.clone(),
            "{time}" => c.color_time.clone(),
            "{memory}" => c.color_memory.clone(),
            _ => c.color_separator.clone(),
        }
    }

    /// Whether a segment has visible content (used to skip empty segments).
    fn segment_has_content(&self, placeholder: &str) -> bool {
        match placeholder {
            "{memory}" => !self.memory_text.is_empty(),
            _ => {
                if let Some(name) = placeholder.strip_prefix("{command_").and_then(|s| s.strip_suffix('}')) {
                    let has_widget = self.hud_config.command_widgets.contains_key(name);
                    let has_output = self.command_outputs.get(name).map_or(false, |o| !o.stdout.is_empty());
                    // Show if widget exists and either has output or format is not just {stdout}
                    has_widget && (has_output || self.hud_config.command_widgets.get(name).map_or(false, |w| w.format != "{stdout}"))
                } else {
                    true
                }
            }
        }
    }

    /// Render segment text for powerline mode.
    /// Returns plain text with minimal ANSI (bold/reset-bold for active tabs only).
    /// The caller applies fg/bg colors around this text.
    fn segment_powerline_text(&self, placeholder: &str) -> String {
        let c = &self.hud_config;
        match placeholder {
            "{mode}" => {
                c.mode_content
                    .get(&self.mode)
                    .cloned()
                    .unwrap_or_else(|| format!("{:?}", self.mode).to_uppercase())
            }
            "{session}" => {
                c.session_format.replace("{name}", &self.session_name)
            }
            "{tabs}" => {
                let mut out = String::new();
                for (i, tab) in self.tabs.iter().enumerate() {
                    if i > 0 {
                        out.push(' ');
                    }
                    if tab.active {
                        out.push_str(&format!("\x1b[1m{}\x1b[22m", tab.name));
                    } else {
                        out.push_str(&tab.name);
                    }
                }
                out
            }
            "{cwd}" => format!("󰉖 {}", self.format_cwd()),
            "{date}" => format!("󰃭 {}", self.format_date()),
            "{time}" => format!("󰥔 {}", self.format_time()),
            "{memory}" => format!("󰍛 {}", self.memory_text),
            _ => {
                if let Some(name) = placeholder.strip_prefix("{command_").and_then(|s| s.strip_suffix('}')) {
                    if let Some(widget) = self.hud_config.command_widgets.get(name) {
                        let output = self.command_outputs.get(name);
                        let stdout = output.map(|o| o.stdout.as_str()).unwrap_or("");
                        let exit_code = output.map(|o| o.exit_code).unwrap_or(0);
                        widget.format
                            .replace("{stdout}", stdout)
                            .replace("{exit_code}", &exit_code.to_string())
                    } else {
                        String::new()
                    }
                } else if let Some(name) = placeholder.strip_prefix("{text_").and_then(|s| s.strip_suffix('}')) {
                    self.hud_config.text_widgets.get(name)
                        .map(|w| w.content.clone())
                        .unwrap_or_default()
                } else {
                    String::new()
                }
            }
        }
    }

    /// Render HUD in powerline mode.
    ///
    /// Left area:  [seg_bg][text_fg] content [seg_fg][next_bg]▶ ...
    /// Right area: ... [prev_bg][seg_fg]◀[seg_bg][text_fg] content
    pub(crate) fn render_hud_powerline(&self, cols: usize) {
        let c = &self.hud_config;
        let hud_bg = c.color_bg.bg();
        let text_fg = c.color_bg.fg();
        let reset = "\x1b[0m";
        let sep_left = c.separator.powerline_left();
        let sep_right = c.separator.powerline_right();

        let left_segs: Vec<&str> = c.format_left.split(" | ")
            .map(str::trim)
            .filter(|p| self.segment_has_content(p))
            .collect();
        let right_segs: Vec<&str> = c.format_right.split(" | ")
            .map(str::trim)
            .filter(|p| self.segment_has_content(p))
            .collect();

        // Build left area
        let mut left = String::new();
        for (i, &seg) in left_segs.iter().enumerate() {
            let seg_color = self.segment_color(seg);
            let seg_fg = seg_color.fg();
            let seg_bg = seg_color.bg();
            let content = self.segment_powerline_text(seg);
            left.push_str(&format!("{seg_bg}{text_fg} {content} "));
            // Arrow: fg = this segment's color, bg = next segment's color (or hud_bg)
            let next_bg = if i + 1 < left_segs.len() {
                self.segment_color(left_segs[i + 1]).bg()
            } else {
                hud_bg.clone()
            };
            left.push_str(&format!("{reset}{seg_fg}{next_bg}{sep_left}"));
        }

        // Build right area
        let mut right = String::new();
        for (i, &seg) in right_segs.iter().enumerate() {
            let seg_color = self.segment_color(seg);
            let seg_fg = seg_color.fg();
            let seg_bg = seg_color.bg();
            let content = self.segment_powerline_text(seg);
            // Arrow: fg = this segment's color, bg = previous area's color (hud_bg or prev seg)
            let prev_bg = if i == 0 {
                hud_bg.clone()
            } else {
                self.segment_color(right_segs[i - 1]).bg()
            };
            right.push_str(&format!("{reset}{prev_bg}{seg_fg}{sep_right}"));
            right.push_str(&format!("{seg_bg}{text_fg} {content} "));
        }
        right.push_str(reset);

        let left_visible = visible_len(&left);
        let right_visible = visible_len(&right);
        let gap = cols.saturating_sub(left_visible + right_visible);

        print!("{hud_bg}{left}{}{right}", " ".repeat(gap));
    }

    pub(crate) fn render_format(&self, format_str: &str, is_right: bool) -> String {
        let c = &self.hud_config;
        let reset = "\x1b[0m";
        let sep_char = if is_right {
            c.separator.minimal_right()
        } else {
            c.separator.minimal_left()
        };
        let sep = format!("{}{}{reset}", c.color_separator.fg(), sep_char);

        let parts: Vec<&str> = format_str.split(" | ").collect();
        let mut out = String::new();

        for (_i, part) in parts.iter().enumerate() {
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

    pub(crate) fn format_cwd(&self) -> String {
        if let Some(name) = self.cwd.file_name() {
            name.to_string_lossy().to_string()
        } else {
            self.cwd.to_string_lossy().to_string()
        }
    }
}
