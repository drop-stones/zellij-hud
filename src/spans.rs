//! Two-pass rendering pipeline for the HUD status bar.
//!
//! Pass 1 (flatten): Recursively expand format strings and widgets into a flat
//! `Vec<Span>` intermediate representation. Each `Span` carries its text and
//! style, with colors either fully resolved or marked as positional references
//! (`PrevBg` / `NextBg`).
//!
//! Pass 2 (resolve + emit): Walk the span list to resolve positional color
//! references, then emit the final ANSI-escaped string.

use crate::config::{Color, WidgetStyle};
use crate::State;

/// Maximum recursion depth for widget references in format templates.
const MAX_DEPTH: u8 = 5;

// ---------------------------------------------------------------------------
// IR types
// ---------------------------------------------------------------------------

/// A color that is either resolved or a positional reference.
#[derive(Clone)]
pub(crate) enum SpanColor {
    /// Fully resolved color (may be Color::None for "no color").
    Concrete(Color),
    /// The background color of the preceding span.
    PrevBg,
    /// The background color of the following span.
    NextBg,
}

/// A styled text fragment — the atomic unit of the intermediate representation.
#[derive(Clone)]
pub(crate) struct Span {
    pub(crate) text: String,
    pub(crate) fg: SpanColor,
    pub(crate) bg: SpanColor,
    pub(crate) attr: String,
}

// ---------------------------------------------------------------------------
// Format string tokenizer
// ---------------------------------------------------------------------------

/// A token from a format string: either literal text or a `{widget_ref}`.
enum Token {
    Literal(String),
    Ref(String), // widget name without braces
}

/// Tokenize a format string into literal and widget-ref tokens.
/// Example: `" hello {name} world "` → [Literal(" hello "), Ref("name"), Literal(" world ")]
fn tokenize(format_str: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut literal = String::new();
    let mut chars = format_str.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            // Collect widget ref name
            let mut name = String::new();
            for inner in chars.by_ref() {
                if inner == '}' {
                    break;
                }
                name.push(inner);
            }
            if !name.is_empty() {
                if !literal.is_empty() {
                    tokens.push(Token::Literal(std::mem::take(&mut literal)));
                }
                tokens.push(Token::Ref(name));
            }
        } else {
            literal.push(ch);
        }
    }
    if !literal.is_empty() {
        tokens.push(Token::Literal(literal));
    }
    tokens
}

// ---------------------------------------------------------------------------
// Pass 1: Flatten
// ---------------------------------------------------------------------------

/// Resolved style for a widget: SpanColor for fg/bg plus attr string.
struct ResolvedStyle {
    fg: SpanColor,
    bg: SpanColor,
    attr: String,
}

impl State {
    /// Resolve a WidgetStyle into SpanColors for the current mode.
    fn resolve_style(&self, style: &WidgetStyle) -> ResolvedStyle {
        let c = &self.hud_config;
        ResolvedStyle {
            fg: resolve_span_color(&style.fg, c, self.mode),
            bg: resolve_span_color(&style.bg, c, self.mode),
            attr: style.attr.clone(),
        }
    }

    /// Flatten a top-level format string (format_left or format_right) into spans.
    pub(crate) fn flatten_format(&self, format_str: &str) -> Vec<Span> {
        let tokens = tokenize(format_str);
        let mut spans = Vec::new();
        for token in tokens {
            match token {
                Token::Literal(text) => {
                    // Top-level literal text has no specific style (bar_bg is applied later)
                    spans.push(Span {
                        text,
                        fg: SpanColor::Concrete(Color::None),
                        bg: SpanColor::Concrete(Color::None),
                        attr: String::new(),
                    });
                }
                Token::Ref(name) => {
                    self.flatten_widget(&name, &mut spans, 0);
                }
            }
        }
        spans
    }

    /// Flatten a widget reference into spans, appending to `out`.
    fn flatten_widget(&self, name: &str, out: &mut Vec<Span>, depth: u8) {
        if depth >= MAX_DEPTH {
            return;
        }
        let c = &self.hud_config;

        match name {
            "mode" => {
                let rs = self.resolve_style(&c.mode_style);
                let mode_text = c
                    .mode_content
                    .get(&self.mode)
                    .cloned()
                    .unwrap_or_else(|| format!("{:?}", self.mode).to_uppercase());
                let content = c.mode_format.replace("{content}", &mode_text);
                self.flatten_format_with_style(&content, &rs, out, depth);
            }
            "session" => {
                let rs = self.resolve_style(&c.session_style);
                let content = c.session_format.replace("{name}", &self.session_name);
                self.flatten_format_with_style(&content, &rs, out, depth);
            }
            "tabs" => {
                self.flatten_tabs(out, depth);
            }
            "cwd" => {
                let rs = self.resolve_style(&c.cwd_style);
                let content = c.cwd_format.replace("{cwd}", &self.format_cwd());
                self.flatten_format_with_style(&content, &rs, out, depth);
            }
            _ => {
                // Strip legacy prefixes
                let bare = name
                    .strip_prefix("command_")
                    .or_else(|| name.strip_prefix("text_"))
                    .unwrap_or(name);

                if self.hud_config.command_widgets.contains_key(bare) {
                    self.flatten_command_widget(bare, out, depth);
                } else if self.hud_config.text_widgets.contains_key(bare) {
                    self.flatten_text_widget(bare, out, depth);
                }
            }
        }
    }

    /// Flatten a format string with a parent style into spans.
    /// Literal text gets the parent style; widget refs are recursively expanded.
    fn flatten_format_with_style(
        &self,
        format_str: &str,
        style: &ResolvedStyle,
        out: &mut Vec<Span>,
        depth: u8,
    ) {
        let tokens = tokenize(format_str);
        for token in tokens {
            match token {
                Token::Literal(text) => {
                    out.push(Span {
                        text,
                        fg: style.fg.clone(),
                        bg: style.bg.clone(),
                        attr: style.attr.clone(),
                    });
                }
                Token::Ref(name) => {
                    self.flatten_widget(&name, out, depth + 1);
                }
            }
        }
    }

    /// Flatten the tabs widget.
    /// Emits zero-width bar_bg anchor spans between tabs so that positional
    /// color refs (prev_bg/next_bg) in tab entry/exit separators resolve
    /// correctly through the bar background.
    fn flatten_tabs(&self, out: &mut Vec<Span>, depth: u8) {
        let c = &self.hud_config;
        let bar_bg = c.resolve_color_with_accent(&c.bar_bg, &c.palette, self.mode);
        let anchor = || Span {
            text: String::new(),
            fg: SpanColor::Concrete(Color::None),
            bg: SpanColor::Concrete(bar_bg.clone()),
            attr: String::new(),
        };

        for (i, tab) in self.tabs.iter().enumerate() {
            // Anchor before each tab: represents the bar_bg gap
            out.push(anchor());

            let (style_def, tab_format) = if tab.active {
                (&c.tab_active_style, &c.tab_active_format)
            } else {
                (&c.tab_inactive_style, &c.tab_inactive_format)
            };
            let rs = self.resolve_style(style_def);

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

            self.flatten_format_with_style(&content, &rs, out, depth);
        }
        // Anchor after last tab
        if !self.tabs.is_empty() {
            out.push(anchor());
        }
    }

    /// Flatten a command widget.
    fn flatten_command_widget(&self, name: &str, out: &mut Vec<Span>, depth: u8) {
        let widget = match self.hud_config.command_widgets.get(name) {
            Some(w) => w,
            None => return,
        };
        let output = match self.command_outputs.get(name) {
            Some(o) => o,
            None => return,
        };

        // Hide widget on failure or empty output
        if output.exit_code != 0 || output.stdout.is_empty() {
            return;
        }

        let rs = self.resolve_style(&widget.style);
        let content = widget
            .format
            .replace("{stdout}", &output.stdout)
            .replace("{exit_code}", &output.exit_code.to_string());
        self.flatten_format_with_style(&content, &rs, out, depth);
    }

    /// Flatten a text widget.
    fn flatten_text_widget(&self, name: &str, out: &mut Vec<Span>, depth: u8) {
        let widget = match self.hud_config.text_widgets.get(name) {
            Some(w) => w,
            None => return,
        };
        let rs = self.resolve_style(&widget.style);
        let content = widget.format.replace("{content}", &widget.content);
        self.flatten_format_with_style(&content, &rs, out, depth);
    }
}

// ---------------------------------------------------------------------------
// Pass 2: Resolve positional refs + emit ANSI
// ---------------------------------------------------------------------------

/// Build ANSI escape for text attributes.
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

/// Resolve positional color references and emit an ANSI string.
/// `bar_bg` is the fallback color at span list boundaries.
pub(crate) fn resolve_and_emit(spans: &mut [Span], bar_bg: &Color) -> String {
    // Forward pass: resolve PrevBg
    let mut last_concrete_bg = bar_bg.clone();
    for span in spans.iter_mut() {
        if let SpanColor::PrevBg = &span.fg {
            span.fg = SpanColor::Concrete(last_concrete_bg.clone());
        }
        if let SpanColor::PrevBg = &span.bg {
            span.bg = SpanColor::Concrete(last_concrete_bg.clone());
        }
        if let SpanColor::Concrete(c) = &span.bg {
            last_concrete_bg = c.clone();
        }
    }

    // Backward pass: resolve NextBg
    let mut next_concrete_bg = bar_bg.clone();
    for span in spans.iter_mut().rev() {
        if let SpanColor::NextBg = &span.fg {
            span.fg = SpanColor::Concrete(next_concrete_bg.clone());
        }
        if let SpanColor::NextBg = &span.bg {
            span.bg = SpanColor::Concrete(next_concrete_bg.clone());
        }
        if let SpanColor::Concrete(c) = &span.bg {
            next_concrete_bg = c.clone();
        }
    }

    // Emit ANSI
    let mut out = String::new();
    for span in spans.iter() {
        if span.text.is_empty() {
            continue;
        }
        let fg_esc = match &span.fg {
            SpanColor::Concrete(c) => c.fg(),
            _ => String::new(), // should not happen after resolution
        };
        let bg_esc = match &span.bg {
            SpanColor::Concrete(c) => c.bg(),
            _ => String::new(),
        };
        let attr_esc = attr_escape(&span.attr);
        out.push_str(&format!(
            "\x1b[0m{}{}{}{}", bg_esc, fg_esc, attr_esc, span.text
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve a color string into a SpanColor, recognizing "prev_bg" and "next_bg".
fn resolve_span_color(
    value: &str,
    config: &crate::config::HudConfig,
    mode: zellij_tile::prelude::InputMode,
) -> SpanColor {
    match value {
        "" => SpanColor::Concrete(Color::None),
        "prev_bg" => SpanColor::PrevBg,
        "next_bg" => SpanColor::NextBg,
        _ => SpanColor::Concrete(config.resolve_color_with_accent(value, &config.palette, mode)),
    }
}
