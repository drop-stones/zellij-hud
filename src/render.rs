use zellij_tile::prelude::InputMode;

use crate::State;

/// Count visible characters in a string, ignoring ANSI escape sequences.
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
            len += 1;
        }
    }
    len
}

impl State {
    pub(crate) fn render_segment(&self, placeholder: &str) -> String {
        let c = &self.hud_config;
        let reset = "\x1b[0m";

        match placeholder {
            "{session}" => {
                format!("{}󰆍 {}{reset}", c.color_session.fg(), self.session_name)
            }
            "{mode}" => {
                format!(
                    "{}{} {}{reset}",
                    c.color_for_mode(self.mode).fg(),
                    self.mode_icon(),
                    format!("{:?}", self.mode).to_uppercase(),
                )
            }
            "{tabs}" => {
                let mut out = String::new();
                for tab in &self.tabs {
                    if tab.active {
                        out.push_str(&format!("{} {} {reset}", c.color_tab_active.fg(), tab.name));
                    } else {
                        out.push_str(&format!(
                            "{} {} {reset}",
                            c.color_tab_inactive.fg(), tab.name
                        ));
                    }
                }
                out
            }
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
            _ => String::new(),
        }
    }

    /// The segment's representative color. Used as bg in powerline mode.
    fn segment_color(&self, placeholder: &str) -> &crate::config::Color {
        let c = &self.hud_config;
        match placeholder {
            "{session}" => &c.color_session,
            "{mode}" => c.color_for_mode(self.mode),
            "{tabs}" => &c.color_tab_active,
            "{cwd}" => &c.color_cwd,
            "{date}" => &c.color_date,
            "{time}" => &c.color_time,
            "{memory}" => &c.color_memory,
            _ => &c.color_separator,
        }
    }

    /// Whether a segment has visible content (used to skip empty segments).
    fn segment_has_content(&self, placeholder: &str) -> bool {
        match placeholder {
            "{memory}" => !self.memory_text.is_empty(),
            _ => true,
        }
    }

    /// Render segment text for powerline mode.
    /// Returns plain text with minimal ANSI (bold/reset-bold for active tabs only).
    /// The caller applies fg/bg colors around this text.
    fn segment_powerline_text(&self, placeholder: &str) -> String {
        match placeholder {
            "{session}" => format!("󰆍 {}", self.session_name),
            "{mode}" => format!("{} {}", self.mode_icon(), format!("{:?}", self.mode).to_uppercase()),
            "{tabs}" => {
                let mut out = String::new();
                for tab in &self.tabs {
                    if tab.active {
                        out.push_str(&format!("\x1b[1m{}\x1b[22m ", tab.name));
                    } else {
                        out.push_str(&format!("{} ", tab.name));
                    }
                }
                out.trim_end().to_string()
            }
            "{cwd}" => format!("󰉖 {}", self.format_cwd()),
            "{date}" => format!("󰃭 {}", self.format_date()),
            "{time}" => format!("󰥔 {}", self.format_time()),
            "{memory}" => format!("󰍛 {}", self.memory_text),
            _ => String::new(),
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
        let sep_left = &c.separator_left;
        let sep_right = &c.separator_right;

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

    pub(crate) fn render_format(&self, format_str: &str) -> String {
        let c = &self.hud_config;
        let reset = "\x1b[0m";
        let sep = format!("{}{}{reset}", c.color_separator.fg(), c.separator);

        let parts: Vec<&str> = format_str.split(" | ").collect();
        let mut out = String::new();

        for (i, part) in parts.iter().enumerate() {
            let trimmed = part.trim();
            out.push_str(&self.render_segment(trimmed));
            if i < parts.len() - 1 {
                out.push_str(&format!(" {sep} "));
            }
        }

        out
    }

    pub(crate) fn mode_icon(&self) -> &str {
        match self.mode {
            InputMode::Normal => "󰍀",
            InputMode::Locked => "󰌾",
            InputMode::Pane => "󰘖",
            InputMode::Tab => "󰓩",
            InputMode::Resize => "󰩨",
            InputMode::Move => "󰆾",
            InputMode::Scroll => "󰠶",
            InputMode::Session => "󱂬",
            InputMode::Search => "󰍉",
            InputMode::RenameTab => "󰏫",
            InputMode::RenamePane => "󰏫",
            InputMode::EnterSearch => "󰍉",
            InputMode::Tmux => "󰰣",
            InputMode::Prompt => "󰘥",
        }
    }

    pub(crate) fn format_cwd(&self) -> String {
        if let Some(name) = self.cwd.file_name() {
            name.to_string_lossy().to_string()
        } else {
            self.cwd.to_string_lossy().to_string()
        }
    }
}
