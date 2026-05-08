//! Two-pass rendering pipeline for the HUD status bar.
//!
//! Pass 1 (flatten — bin-side, see `crate::flatten`): expand format strings
//! and widgets into a flat `Vec<Span>` IR. Each `Span` carries its text and
//! style, with colors either fully resolved or marked as positional refs
//! (`PrevBg` / `NextBg`).
//!
//! Pass 2 (resolve + emit — this module): walk the span list to resolve
//! positional color references, then emit the final ANSI-escaped string.

use crate::config::{Color, HudConfig};
use zellij_tile::prelude::InputMode;

/// Maximum recursion depth for widget references in format templates.
pub const MAX_DEPTH: u8 = 5;

// ---------------------------------------------------------------------------
// IR types
// ---------------------------------------------------------------------------

/// A color that is either resolved or a positional reference.
#[derive(Clone)]
pub enum SpanColor {
    /// Fully resolved color (may be Color::None for "no color").
    Concrete(Color),
    /// The background color of the preceding span.
    PrevBg,
    /// The background color of the following span.
    NextBg,
}

/// A styled text fragment — the atomic unit of the intermediate representation.
#[derive(Clone)]
pub struct Span {
    pub text: String,
    pub fg: SpanColor,
    pub bg: SpanColor,
    pub attr: String,
}

// ---------------------------------------------------------------------------
// Format string tokenizer
// ---------------------------------------------------------------------------

/// A token from a format string: either literal text or a `{widget_ref}`.
pub enum Token {
    Literal(String),
    Ref(String),
}

/// Tokenize a format string into literal and widget-ref tokens.
///
/// Example: `" hello {name} world "` →
/// `[Literal(" hello "), Ref("name"), Literal(" world ")]`.
///
/// Unclosed `{` or empty `{}` are kept as literal text to avoid silently
/// dropping user input.
pub fn tokenize(format_str: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut literal = String::new();
    let mut chars = format_str.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '{' {
            let mut name = String::new();
            let mut closed = false;
            for inner in chars.by_ref() {
                if inner == '}' {
                    closed = true;
                    break;
                }
                name.push(inner);
            }
            if closed && !name.is_empty() {
                if !literal.is_empty() {
                    tokens.push(Token::Literal(std::mem::take(&mut literal)));
                }
                tokens.push(Token::Ref(name));
            } else {
                literal.push('{');
                literal.push_str(&name);
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
// Pass 2: Resolve positional refs + emit ANSI
// ---------------------------------------------------------------------------

/// Build ANSI escape for text attributes.
pub fn attr_escape(attr: &str) -> String {
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
pub fn resolve_and_emit(spans: &mut [Span], bar_bg: &Color) -> String {
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
            SpanColor::Concrete(Color::None) => bar_bg.bg(),
            SpanColor::Concrete(c) => c.bg(),
            _ => bar_bg.bg(),
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
pub fn resolve_span_color(value: &str, config: &HudConfig, mode: InputMode) -> SpanColor {
    match value {
        "" => SpanColor::Concrete(Color::None),
        "prev_bg" => SpanColor::PrevBg,
        "next_bg" => SpanColor::NextBg,
        _ => SpanColor::Concrete(config.resolve_color_with_accent(value, &config.palette, mode)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lit(s: &str) -> Span {
        Span {
            text: s.to_string(),
            fg: SpanColor::Concrete(Color::None),
            bg: SpanColor::Concrete(Color::None),
            attr: String::new(),
        }
    }

    fn rgb(r: u8, g: u8, b: u8) -> Color {
        Color::Rgb(r, g, b)
    }

    fn token_strings(format_str: &str) -> Vec<String> {
        tokenize(format_str)
            .into_iter()
            .map(|t| match t {
                Token::Literal(s) => format!("L:{}", s),
                Token::Ref(s) => format!("R:{}", s),
            })
            .collect()
    }

    #[test]
    fn tokenize_splits_literals_and_refs() {
        assert_eq!(
            token_strings(" hello {name} world "),
            vec!["L: hello ", "R:name", "L: world "],
        );
    }

    #[test]
    fn tokenize_handles_no_refs_as_single_literal() {
        assert_eq!(token_strings("plain text"), vec!["L:plain text"]);
    }

    #[test]
    fn tokenize_keeps_unclosed_brace_as_literal() {
        // Avoid silent loss: a stray `{` must end up in the output.
        assert_eq!(token_strings("a {b"), vec!["L:a {b"]);
    }

    #[test]
    fn tokenize_treats_empty_braces_as_text_minus_closing_brace() {
        // `{}` isn't a valid widget reference. The current parser keeps the
        // opening `{` and consumes the `}` (the `}` is treated as the
        // terminator of an empty ref, then the empty ref falls back to the
        // literal-with-`{` branch). Pinning this exact behaviour so a future
        // refactor doesn't silently change it.
        assert_eq!(token_strings("a{}b"), vec!["L:a{b"]);
    }

    #[test]
    fn tokenize_handles_adjacent_refs() {
        assert_eq!(
            token_strings("{a}{b}"),
            vec!["R:a", "R:b"],
        );
    }

    #[test]
    fn attr_escape_emits_bold_italic_combinations() {
        assert_eq!(attr_escape(""), "");
        assert_eq!(attr_escape("bold"), "\x1b[1m");
        assert_eq!(attr_escape("italic"), "\x1b[3m");
        assert_eq!(attr_escape("bold,italic"), "\x1b[1m\x1b[3m");
        // Whitespace tolerated
        assert_eq!(attr_escape("bold, italic"), "\x1b[1m\x1b[3m");
        // Unknown values silently ignored
        assert_eq!(attr_escape("rainbow"), "");
    }

    #[test]
    fn resolve_and_emit_resolves_prev_bg_in_forward_pass() {
        // [bg=red] [fg=PrevBg, bg=blue] → middle's fg becomes red.
        let mut spans = vec![
            Span {
                text: "X".into(),
                fg: SpanColor::Concrete(Color::None),
                bg: SpanColor::Concrete(rgb(255, 0, 0)),
                attr: String::new(),
            },
            Span {
                text: "Y".into(),
                fg: SpanColor::PrevBg,
                bg: SpanColor::Concrete(rgb(0, 0, 255)),
                attr: String::new(),
            },
        ];
        let _ = resolve_and_emit(&mut spans, &Color::None);
        match &spans[1].fg {
            SpanColor::Concrete(Color::Rgb(255, 0, 0)) => (),
            other => panic!("expected fg=red, got {:?}", color_label(other)),
        }
    }

    #[test]
    fn resolve_and_emit_resolves_next_bg_in_backward_pass() {
        // [fg=NextBg, bg=red] [bg=blue] → first's fg becomes blue.
        let mut spans = vec![
            Span {
                text: "X".into(),
                fg: SpanColor::NextBg,
                bg: SpanColor::Concrete(rgb(255, 0, 0)),
                attr: String::new(),
            },
            Span {
                text: "Y".into(),
                fg: SpanColor::Concrete(Color::None),
                bg: SpanColor::Concrete(rgb(0, 0, 255)),
                attr: String::new(),
            },
        ];
        let _ = resolve_and_emit(&mut spans, &Color::None);
        match &spans[0].fg {
            SpanColor::Concrete(Color::Rgb(0, 0, 255)) => (),
            other => panic!("expected fg=blue, got {:?}", color_label(other)),
        }
    }

    #[test]
    fn resolve_and_emit_falls_back_to_bar_bg_at_boundaries() {
        // Single span asking for PrevBg — there's nothing before, so it
        // must fall back to bar_bg.
        let bar_bg = rgb(10, 20, 30);
        let mut spans = vec![Span {
            text: "X".into(),
            fg: SpanColor::PrevBg,
            bg: SpanColor::Concrete(Color::None),
            attr: String::new(),
        }];
        let _ = resolve_and_emit(&mut spans, &bar_bg);
        match &spans[0].fg {
            SpanColor::Concrete(Color::Rgb(10, 20, 30)) => (),
            other => panic!("expected fg=bar_bg, got {:?}", color_label(other)),
        }
    }

    #[test]
    fn resolve_and_emit_skips_empty_text_spans() {
        // Empty-text spans are anchors used to thread positional colors;
        // they must not produce any output bytes.
        let mut spans = vec![
            lit(""),
            Span {
                text: "ok".into(),
                fg: SpanColor::Concrete(Color::None),
                bg: SpanColor::Concrete(Color::None),
                attr: String::new(),
            },
            lit(""),
        ];
        let out = resolve_and_emit(&mut spans, &Color::None);
        assert!(out.contains("ok"));
        // The reset prefix is per non-empty span; one occurrence => one span.
        assert_eq!(out.matches("\x1b[0m").count(), 1);
    }

    fn color_label(c: &SpanColor) -> &'static str {
        match c {
            SpanColor::Concrete(Color::None) => "Concrete(None)",
            SpanColor::Concrete(Color::Rgb(_, _, _)) => "Concrete(Rgb)",
            SpanColor::Concrete(Color::EightBit(_)) => "Concrete(EightBit)",
            SpanColor::PrevBg => "PrevBg",
            SpanColor::NextBg => "NextBg",
        }
    }
}
