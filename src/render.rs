use zellij_tile::prelude::InputMode;

use crate::State;

impl State {
    pub(crate) fn render_segment(&self, placeholder: &str) -> String {
        let c = &self.hud_config;
        let bg = &c.color_bg;
        let reset = "\x1b[0m";

        match placeholder {
            "{session}" => {
                format!("{}󰆍 {}{reset}{bg}", c.color_session, self.session_name)
            }
            "{mode}" => {
                format!(
                    "{}{} {}{reset}{bg}",
                    c.color_for_mode(self.mode),
                    self.mode_icon(),
                    format!("{:?}", self.mode).to_uppercase(),
                )
            }
            "{tabs}" => {
                let mut out = String::new();
                for tab in &self.tabs {
                    if tab.active {
                        out.push_str(&format!("{} {} {reset}{bg}", c.color_tab_active, tab.name));
                    } else {
                        out.push_str(&format!(
                            "{} {} {reset}{bg}",
                            c.color_tab_inactive, tab.name
                        ));
                    }
                }
                out
            }
            "{cwd}" => {
                format!("{}󰉖 {}{reset}{bg}", c.color_cwd, self.format_cwd())
            }
            "{date}" => {
                format!("{}󰃭 {}{reset}{bg}", c.color_date, self.format_date())
            }
            "{time}" => {
                format!("{}󰥔 {}{reset}{bg}", c.color_time, self.format_time())
            }
            "{memory}" => {
                if self.memory_text.is_empty() {
                    String::new()
                } else {
                    format!("{}󰍛 {}{reset}{bg}", c.color_memory, self.memory_text)
                }
            }
            _ => String::new(),
        }
    }

    pub(crate) fn render_format(&self, format_str: &str) -> String {
        let c = &self.hud_config;
        let bg = &c.color_bg;
        let reset = "\x1b[0m";
        let sep = format!("{}{}{reset}", c.color_separator, c.separator);

        let parts: Vec<&str> = format_str.split(" | ").collect();
        let mut out = String::new();

        for (i, part) in parts.iter().enumerate() {
            let trimmed = part.trim();
            out.push_str(&self.render_segment(trimmed));
            if i < parts.len() - 1 {
                out.push_str(&format!(" {sep}{bg} "));
            }
        }

        out
    }

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

    pub(crate) fn mode_icon(&self) -> &str {
        match self.mode {
            InputMode::Normal => "󰰓",
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
