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

/// Maximum recursion depth for widget references in format templates.
const MAX_WIDGET_DEPTH: u8 = 5;

/// Extract {placeholder} tokens from a format string.
fn extract_placeholders(format_str: &str) -> Vec<String> {
    let mut placeholders = Vec::new();
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
    placeholders
}

/// Resolve widget references in a string: replace {NAME} tokens with rendered widgets.
/// Leaves unrecognized tokens as-is.
fn resolve_widget_refs(state: &State, text: &str, depth: u8) -> String {
    if depth >= MAX_WIDGET_DEPTH {
        return text.to_string();
    }
    let mut result = text.to_string();
    let placeholders = extract_placeholders(text);
    for ph in &placeholders {
        let rendered = state.render_segment_with_depth(ph, depth + 1);
        if !rendered.is_empty() {
            result = result.replacen(ph, &rendered, 1);
        }
    }
    result
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
        self.render_segment_with_depth(placeholder, 0)
    }

    /// Render a segment with recursion depth tracking.
    fn render_segment_with_depth(&self, placeholder: &str, depth: u8) -> String {
        if depth >= MAX_WIDGET_DEPTH {
            return String::new();
        }

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
                let content = resolve_widget_refs(self, &content, depth);
                format!("{bg}{fg}{attr}{content}{reset}")
            }
            "{session}" => {
                let (fg, bg, attr) = self.style_escapes(&c.session_style);
                let content = c.session_format.replace("{name}", &self.session_name);
                let content = resolve_widget_refs(self, &content, depth);
                format!("{bg}{fg}{attr}{content}{reset}")
            }
            "{tabs}" => {
                self.render_tabs_with_depth(depth)
            }
            "{cwd}" => {
                let (fg, bg, attr) = self.style_escapes(&c.cwd_style);
                let content = c.cwd_format.replace("{cwd}", &self.format_cwd());
                let content = resolve_widget_refs(self, &content, depth);
                format!("{bg}{fg}{attr}{content}{reset}")
            }
            _ => {
                let name = placeholder.strip_prefix('{').and_then(|s| s.strip_suffix('}'));
                if let Some(name) = name {
                    // Strip legacy prefixes for backward compat
                    let bare = name.strip_prefix("command_")
                        .or_else(|| name.strip_prefix("text_"))
                        .unwrap_or(name);
                    if self.hud_config.command_widgets.contains_key(bare) {
                        self.render_command_widget_with_depth(bare, depth)
                    } else if self.hud_config.text_widgets.contains_key(bare) {
                        self.render_text_widget_with_depth(bare, depth)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                }
            }
        }
    }

    /// Render tabs widget.
    fn render_tabs_with_depth(&self, depth: u8) -> String {
        let c = &self.hud_config;
        let reset = "\x1b[0m";
        let mut out = String::new();

        for (i, tab) in self.tabs.iter().enumerate() {
            let style = if tab.active {
                &c.tab_active_style
            } else {
                &c.tab_inactive_style
            };
            let (fg, bg, attr) = self.style_escapes(style);
            let tab_format = if tab.active {
                &c.tab_active_format
            } else {
                &c.tab_inactive_format
            };
            let mut content = tab_format
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

            // Resolve widget references in tab format
            let content = resolve_widget_refs(self, &content, depth);
            out.push_str(&format!("{bg}{fg}{attr}{content}{reset}"));
        }

        out
    }

    /// Render a user-defined command widget.
    fn render_command_widget_with_depth(&self, name: &str, depth: u8) -> String {
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

        // Resolve widget references in command format
        let content = resolve_widget_refs(self, &content, depth);

        let reset = "\x1b[0m";
        let (fg, bg, attr) = self.style_escapes(&widget.style);
        format!("{bg}{fg}{attr}{content}{reset}")
    }

    /// Render a user-defined text widget.
    fn render_text_widget_with_depth(&self, name: &str, depth: u8) -> String {
        let widget = match self.hud_config.text_widgets.get(name) {
            Some(w) => w,
            None => return String::new(),
        };
        let reset = "\x1b[0m";
        let (fg, bg, attr) = self.style_escapes(&widget.style);
        let content = widget.format.replace("{content}", &widget.content);

        // Resolve widget references in text format
        let content = resolve_widget_refs(self, &content, depth);
        format!("{bg}{fg}{attr}{content}{reset}")
    }

    /// Unified format renderer. Extracts {placeholder} tokens, renders each segment,
    /// and concatenates them.
    pub(crate) fn render_format(&self, format_str: &str) -> String {
        let placeholders = extract_placeholders(format_str);
        let mut out = String::new();
        for ph in &placeholders {
            let rendered = self.render_segment(ph);
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
