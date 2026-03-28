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
            "{cwd}" => {
                let (fg, bg, attr) = self.style_escapes(&c.cwd_style);
                let content = format!("󰉖 {}", self.format_cwd());
                if bg.is_empty() {
                    format!("{fg}{attr}{content}{reset}")
                } else {
                    format!("{bg}{fg}{attr} {content} {reset}")
                }
            }
            "{date}" => {
                let (fg, bg, attr) = self.style_escapes(&c.date_style);
                let content = format!("󰃭 {}", self.format_date());
                if bg.is_empty() {
                    format!("{fg}{attr}{content}{reset}")
                } else {
                    format!("{bg}{fg}{attr} {content} {reset}")
                }
            }
            "{time}" => {
                let (fg, bg, attr) = self.style_escapes(&c.time_style);
                let content = format!("󰥔 {}", self.format_time());
                if bg.is_empty() {
                    format!("{fg}{attr}{content}{reset}")
                } else {
                    format!("{bg}{fg}{attr} {content} {reset}")
                }
            }
            "{memory}" => {
                if self.memory_text.is_empty() {
                    String::new()
                } else {
                    let (fg, bg, attr) = self.style_escapes(&c.memory_style);
                    let content = format!("󰍛 {}", self.memory_text);
                    if bg.is_empty() {
                        format!("{fg}{attr}{content}{reset}")
                    } else {
                        format!("{bg}{fg}{attr} {content} {reset}")
                    }
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

    pub(crate) fn render_format(&self, format_str: &str, is_right: bool) -> String {
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

    pub(crate) fn format_cwd(&self) -> String {
        if let Some(name) = self.cwd.file_name() {
            name.to_string_lossy().to_string()
        } else {
            self.cwd.to_string_lossy().to_string()
        }
    }
}
