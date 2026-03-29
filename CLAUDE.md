# zellij-hud

On-demand floating status bar + which-key tooltip for zellij. Hidden in base mode (zero footprint), appears as floating panes on mode change.

## Language

- All code comments, commit messages, and documentation in English
- Conversations may be in Japanese

## Build

```sh
nix develop -c cargo build --release --target wasm32-wasip1
```

Load via command (NOT layout — layout causes "Compiling WASM" hang):
```sh
zellij plugin -- file:~/repos/zellij/zellij-hud/target/wasm32-wasip1/release/zellij-hud.wasm
```

## Architecture: Daemon + HUD + Tooltip

Single WASM binary (`zellij-tile = "0.44.0"`), three roles distinguished by `Role` enum:

### Source files

- `src/main.rs` — Plugin state, `Role` enum, event handling, role dispatch
- `src/config.rs` — Configuration parsing, `Color` type, `ThemePalette`, theme presets, `WidgetStyle`, `StyleDefaults`
- `src/render.rs` — Unified HUD status bar rendering (composable widget architecture)
- `src/tooltip.rs` — Which-key tooltip rendering, dynamic resize
- `src/keybinds.rs` — Keybinding extraction from `ModeInfo`
- `src/action_types.rs` — `ActionType` categorization for icon colors
- `src/spawn.rs` — HUD/Tooltip spawning via `pipe_message_to_plugin`
- `src/commands.rs` — External command execution (git branch, etc.)
- `src/datetime.rs` — Date/time formatting

### Roles

1. **Daemon**: Loaded by user, hides itself. Spawns HUD and Tooltip on non-base mode.
2. **HUD**: Floating status bar (session, mode, datetime). Closes itself on base mode.
3. **Tooltip**: Floating which-key keybinding hints. Dynamically resizes. Closes on base mode.

### Why this pattern?

- `show_self(true)` steals focus — unusable for passive display
- `hide_self()` blocks event delivery — daemon must be separate from HUD
- `pipe_message_to_plugin` with `zellij:OWN_URL` spawns new instance without stealing focus

## Key Technical Notes

### Rendering

- Use `print!()` not `println!()` — println causes blank output
- HUD is borderless (height=1); tooltip keeps frame (height = content + 2 border rows)
- `Text` API `color_range()` uses **character indices** (`.chars().count()`), not byte indices
- `print_text(text)` for theme-colored output with `Text::new().opaque()`
- `Color` enum (`None`, `Rgb`, `EightBit`) with `.fg()` and `.bg()` methods for ANSI escapes
- Composable widget architecture: all widgets (built-in and user-defined) render through the same uniform pipeline
- Style presets (`style "minimal"` / `style "powerline"`) control format strings and default text widgets — no rendering branches
- Separators are regular text widgets composed in format strings, not special-cased rendering logic
- Style restore mechanism: `resolve_widget_refs` re-applies parent style after each child widget reset
- HUD background color: `\x1b[0m` resets are replaced with `\x1b[0m{bg}` to maintain background across segments

### Tab following

- `get_plugin_ids()` is synchronous — call in `load()`
- Use `tabs.iter().position(|t| t.active)` (iterator position, NOT `TabInfo.position` field)
- `break_panes_to_tab_with_index` moves HUD to active tab

### Timer

- `set_timeout(1.0)` + `Event::Timer(_)` for 1-second clock updates
- `std::time::SystemTime::now()` works in wasm32-wasip1

### Frame

- `rename_plugin_pane(id, "")` clears frame title
- `FloatingPaneCoordinates::new()` 6th arg `borderless: Option<bool>` — HUD uses `Some(true)`, tooltip uses `None`
- `set_pane_borderless(pane_id, borderless)` available for runtime control

### Permissions

- Daemon/HUD both need: `ReadApplicationState`, `ChangeApplicationState`, `MessageAndLaunchOtherPlugins`, `RunCommands`

## Configuration Spec

Design goals: composable widget architecture with zero rendering branches. All visual differences between styles (minimal, powerline) are expressed purely through config (format strings + widget definitions). No implicit behavior in the render path.

### Global

```kdl
theme "system"              // "system" (default) | "tokyonight" | "catppuccin-mocha" | "nord" | "gruvbox-dark"
style "minimal"             // "minimal" (default) | "powerline" — preset for format strings, widget styles, and separator widgets
enable_status_bar "true"
enable_tooltip "true"
```

### Layout

```kdl
format_left "{mode}{sep}{session}{sep}{tabs}"       // minimal default
format_right "{cwd}{git_branch}{sep}{memory}{sep}{time}"
bar_bg "bg"                                          // status bar background color
```

- `{NAME}` tokens reference built-in or user-defined widgets
- Widget composition: widgets can reference other widgets in their format templates (recursive, max depth 5)
- Spaces between tokens in format strings are literal (not ignored)

### Palette (10 colors, set by theme, individually overridable)

```kdl
palette_fg "#c0caf5"
palette_bg "#1a1b26"
palette_dim "#565f89"
palette_red "#f7768e"
palette_green "#9ece6a"
palette_yellow "#e0af68"
palette_blue "#7aa2f7"
palette_magenta "#bb9af7"
palette_cyan "#7dcfff"
palette_orange "#ff9e64"
```

Color values: palette name (`"blue"`, `"dim"`), hex (`"#7aa2f7"`), 8-bit (`"8bit:123"`), or `"accent"` (mode-dependent).

### Mode accent color

Per-mode accent color. Widgets using `"accent"` in fg/bg change color based on current mode.

```kdl
mode_accent_normal "green"
mode_accent_locked "red"
mode_accent_resize "yellow"
mode_accent_pane "blue"
mode_accent_tab "blue"
mode_accent_scroll "cyan"
mode_accent_search "magenta"
mode_accent_enter_search "magenta"
mode_accent_rename_tab "yellow"
mode_accent_rename_pane "yellow"
mode_accent_session "cyan"
mode_accent_move "orange"
mode_accent_prompt "cyan"
mode_accent_tmux "orange"
```

### Widget style keys (common pattern)

All widgets share these style keys (3 keys per widget):

```
{widget}_fg "{color}"          // foreground: palette name | hex | "accent"
{widget}_bg "{color}"          // background: same. Empty = no bg (inherits bar_bg)
{widget}_attr "{decorations}"  // "bold" | "italic" | "bold,italic" | ""
```

### Built-in widgets

#### mode

```kdl
mode_fg "accent"           // minimal default
mode_bg ""
mode_attr "bold"
mode_format " {content} "  // template with {content} placeholder

// Per-mode display text (new keys; old mode_normal etc. accepted as fallback)
mode_content_normal "󰍀 NORMAL"
mode_content_locked "󰌾 LOCKED"
mode_content_pane "󰘖 PANE"
mode_content_tab "󰓩 TAB"
mode_content_resize "󰩨 RESIZE"
mode_content_move "󰆾 MOVE"
mode_content_scroll "󰠶 SCROLL"
mode_content_search "󰍉 SEARCH"
mode_content_enter_search "󰍉 SEARCH"
mode_content_rename_tab "󰏫 RENAME TAB"
mode_content_rename_pane "󰏫 RENAME PANE"
mode_content_session "󱂬 SESSION"
mode_content_prompt "󰘥 PROMPT"
mode_content_tmux "󰰣 TMUX"
```

#### session

```kdl
session_fg "cyan"
session_bg ""
session_attr ""
session_format " 󰆍 {name} "   // template with {name} placeholder
```

#### tabs

```kdl
tab_active_fg "fg"
tab_active_bg ""
tab_active_attr "bold"
tab_inactive_fg "dim"
tab_inactive_bg ""
tab_inactive_attr ""
tab_format " {name}"              // sets both active and inactive (fallback)
tab_active_format " {name}"      // overrides tab_format for active tabs
tab_inactive_format " {name}"    // overrides tab_format for inactive tabs
tab_sync_indicator "🔗"
tab_fullscreen_indicator "⛶"
```

Placeholders: `{name}`, `{index}`, `{sync_indicator}`, `{fullscreen_indicator}`, plus `{WIDGET_NAME}` refs.

Tab formats support widget references for powerline arrows (e.g., `"{ta_in} {name} {ta_out}"`).

#### cwd

```kdl
cwd_fg "cyan"
cwd_bg ""
cwd_attr ""
cwd_format " 󰉋 {cwd} "   // template with {cwd} placeholder
```

### User-defined widgets

#### command widget (shell command execution)

```kdl
NAME_command "..."          // shell command to run
NAME_fg "..."
NAME_bg ""
NAME_attr ""
NAME_format "{stdout}"      // template with {stdout}, {exit_code} placeholders
NAME_interval "10"          // execution interval in seconds
```

Hidden when command fails (exit_code != 0) or stdout is empty.

Legacy `command_NAME_*` prefix also accepted for backward compat.

**Preset command widgets** (overridable): `time`, `date`, `memory`, `git_branch`

#### text widget (static text)

```kdl
NAME_content "..."          // static content string
NAME_fg "..."
NAME_bg ""
NAME_attr ""
NAME_format "{content}"     // template with {content} placeholder
```

Legacy `text_NAME_*` prefix also accepted for backward compat.

**Preset text widgets** (defined by style preset, overridable):
- minimal: `sep` (thin `|` divider)
- powerline: `s_ms`, `s_sb`, `s_cg`, `s_gm`, `s_mt` (segment separators), `ta_in`, `ta_out`, `ti_in`, `ti_out` (tab arrows)

#### Widget type detection

- `NAME_command` key present → command widget
- `NAME_content` key present → text widget
- Widget names must not match reserved prefixes: `mode`, `session`, `tab_active`, `tab_inactive`, `tabs`, `cwd`, `bar`, `tooltip`, `palette`, `format`, `enable`, `theme`, `style`, `base_mode`, `mode_accent`, `mode_content`

### Tooltip

```kdl
tooltip_key_color "cyan"
tooltip_separator_color "dim"
tooltip_description_color "fg"
tooltip_mode_color "accent"
tooltip_bg ""
tooltip_border_color "dim"
tooltip_separator "➜"
tooltip_position "bottom-right"    // "bottom-right" | "bottom-left" | "top-right" | "top-left"
tooltip_title "{mode}"
tooltip_border "true"
```

### Key count

| Category | Keys |
|---|---|
| Global | 4 (theme, style, enable_status_bar, enable_tooltip) |
| Layout | 3 (format_left, format_right, bar_bg) |
| Palette | 10 |
| Mode accent | 14 |
| mode widget | 18 (3 style + 1 format + 14 content) |
| session | 4 (3 style + 1 format) |
| tabs | 9 (6 style + 3 format/indicator) |
| cwd | 4 (3 style + 1 format) |
| command (per NAME) | 6 |
| text (per NAME) | 5 |
| tooltip | 10 |
| **Total** | **~76 + 6-11/user-defined widget** |

### Base mode detection

- Auto-detect from keybindings: count `SwitchToMode` transitions, the most frequent target is the home mode
- `base_mode "locked"` / `base_mode "normal"` for explicit override

### Theme presets

Themes provide: palette colors + mode accent colors. Style presets provide: format strings + widget styles + default text widgets.

Available themes: system (default, uses zellij's palette), tokyonight, catppuccin-mocha, nord, gruvbox-dark.

System theme: `bg` = `ribbon_unselected.base` (maps to `palette.black` in zellij).

## TODO

### Rendering

- [ ] **Positional color references**: Allow widgets to reference adjacent widgets' colors (e.g., `current_bg`, `right_bg`) for separator definitions. Would make powerline separators fully generic without hardcoded color values. `current_bg` = the bg of the parent widget containing this widget ref. `right_bg` = the bg of the next widget in the same format string. Requires 2-pass rendering (first pass resolves widget bg colors, second pass renders with positional refs). Complexity: empty widgets (e.g., git_branch outside a repo) must be skipped when resolving adjacency, and nested contexts (widget refs inside tab formats) make "adjacent" ambiguous.

### Tooltip

- [ ] Multi-column layout (future: `tooltip_columns "auto"`)

### Documentation

- [ ] Update `README.md` with current configuration spec and usage examples
- [ ] Update `../zellij-hud.wiki/` (Configuration.md, Architecture.md, etc.) to reflect composable widget architecture and current config keys

## Upstream proposals (zellij)

### `pipe_message_to_plugin` spawn target tab

`pipe_message_to_plugin` with `zellij:OWN_URL` always places the new pane on the **first connected client's active tab** (`cli_client_id = None` in the internal flow). This causes a brief flash on wrong tabs in multi-client scenarios, because the plugin must `break_panes_to_tab_with_index` after spawn to move it.

**Proposal**: Add a target tab field to `MessageToPlugin` (e.g. `new_plugin_instance_should_open_in_tab(tab_index)`) so plugins can specify the destination tab at spawn time, avoiding the post-spawn move and the flash.

### ~~Per-pane borderless floating panes~~ (resolved in v0.44.0)

Resolved: zellij 0.44.0 added `FloatingPaneCoordinates::borderless` and `set_pane_borderless()` API.
