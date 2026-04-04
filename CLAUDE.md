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
- `src/render.rs` — HUD render entry point (`render_format`), `visible_len`
- `src/spans.rs` — Two-pass rendering: `Span`/`SpanColor` IR, flatten (pass 1), resolve + emit (pass 2)
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
- Style presets (`style "simple"` / `"minimal"` / `"powerline"` / `"bubble"`) control format strings and default text widgets — no rendering branches
- Separators are regular text widgets composed in format strings, not special-cased rendering logic
- Two-pass rendering pipeline (`src/spans.rs`): Pass 1 flattens format strings into `Vec<Span>` IR, Pass 2 resolves positional color refs and emits ANSI
- Positional color refs: `prev_bg` / `next_bg` as special color values — forward pass resolves `prev_bg`, backward pass resolves `next_bg`, `bar_bg` as fallback
- Tabs emit zero-width bar_bg anchor spans so positional refs resolve correctly at tab entry/exit boundaries
- `format_center` uses absolute centering: left_gap/right_gap computed to place center content at exact midpoint of the bar

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

Design goals: composable widget architecture with zero rendering branches. All visual differences between styles (simple, minimal, powerline, bubble) are expressed purely through config (format strings + widget definitions). No implicit behavior in the render path.

### Global

```kdl
theme "system"              // "system" (default) | "tokyonight" | "catppuccin-mocha" | "nord" | "gruvbox-dark"
style "simple"              // "simple" (default) | "minimal" | "powerline" | "bubble"
enable_status_bar "true"
enable_tooltip "true"
```

### Layout

```kdl
format_left "{mode}{sep}{session}{sep}{tabs}"       // simple default
format_center ""                                     // absolute-center section (used by minimal for centered tabs)
format_right "{cwd}{git_branch}{sep}{memory}{sep}{time}"
bar_bg "bg"                                          // status bar background color
```

- `{NAME}` tokens reference built-in or user-defined widgets
- Widget composition: widgets can reference other widgets in their format templates (recursive, max depth 5)
- Spaces between tokens in format strings are literal (not ignored)
- `format_center` content is placed at the exact horizontal midpoint; left/right sections fill remaining space

### Palette (12 colors, set by theme, individually overridable)

```kdl
palette_fg "#c0caf5"
palette_bg "#1a1b26"
palette_dim "#565f89"
palette_surface "#24283b"          // widget background layer (bg < surface < surface_bright)
palette_surface_bright "#292e42"   // active/highlighted widget background
palette_red "#f7768e"
palette_green "#9ece6a"
palette_yellow "#e0af68"
palette_blue "#7aa2f7"
palette_magenta "#bb9af7"
palette_cyan "#7dcfff"
palette_orange "#ff9e64"
```

Color values: palette name (`"blue"`, `"dim"`, `"surface"`, `"surface_bright"`), hex (`"#7aa2f7"`), 8-bit (`"8bit:123"`), `"accent"` (mode-dependent), `"prev_bg"` (bg of preceding rendered text), or `"next_bg"` (bg of following rendered text).

System theme: surface colors auto-computed from bg via `lighten_color` helper (+10 for surface, +20 for surface_bright).

### Mode accent color

Per-mode accent color. Widgets using `"accent"` in fg/bg change color based on current mode.

```kdl
mode_accent_normal "blue"
mode_accent_locked "red"
mode_accent_resize "yellow"
mode_accent_pane "cyan"
mode_accent_tab "green"
mode_accent_scroll "magenta"
mode_accent_search "magenta"
mode_accent_enter_search "magenta"
mode_accent_rename_tab "yellow"
mode_accent_rename_pane "yellow"
mode_accent_session "cyan"
mode_accent_move "orange"
mode_accent_prompt "green"
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
mode_fg "accent"           // simple default
mode_bg ""
mode_attr "bold"
mode_format " {content} "  // template with {content} placeholder

// Per-mode display text
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

Style presets can override mode_content (e.g., minimal uses lowercase: `"󰍀 normal"`).

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

// Inter-tab separator (rendered between consecutive tabs)
tab_separator ""                   // e.g., " • " for minimal style
tab_separator_fg "dim"
tab_separator_bg ""
tab_separator_attr ""
```

Placeholders: `{name}`, `{index}`, `{sync_indicator}`, `{fullscreen_indicator}`, plus `{WIDGET_NAME}` refs.

Tab formats support widget references for powerline arrows (e.g., `"{pl_right} {name} {pl_right}"`).

##### Tab sub-placeholder styles and formats

Each tab sub-placeholder (index, name, sync, fullscreen) can have its own style override and format template. If no style override is set, the parent tab style is used. When a style override has empty fields (e.g., bg=""), those fields inherit from the parent tab style.

```kdl
// Index sub-placeholder (active/inactive)
tab_active_index_fg "bg"
tab_active_index_bg "blue"
tab_active_index_attr "bold"
tab_active_index_format "{content} "    // format template wrapping the index value

tab_inactive_index_fg "bg"
tab_inactive_index_bg "dim"
tab_inactive_index_attr ""
tab_inactive_index_format "{content} "

// Name sub-placeholder (active/inactive)
tab_active_name_format " {content}"     // format template wrapping the tab name
tab_inactive_name_format " {content}"

// Sync indicator sub-placeholder (active/inactive)
tab_active_sync_fg "accent"            // accent fg, bg/attr inherited from tab style
tab_active_sync_format " {content} "
tab_inactive_sync_format " {content} "

// Fullscreen indicator sub-placeholder (active/inactive)
tab_active_fullscreen_fg "accent"
tab_active_fullscreen_format " {content} "
tab_inactive_fullscreen_format " {content} "
```

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
- simple: `sep` (thin `|` divider)
- powerline: `pl_right`, `pl_left`, `pl_thin` (arrow separators), `ta_in`, `ta_out`, `ti_in`, `ti_out` (tab arrows)
- bubble: `pill_left`, `pill_right` (rounded pill edges), `gap` (bar-bg spacer), `sess_icon`, `cwd_icon`, `git_icon`, `mem_icon`, `time_icon`, `date_icon` (two-tone icon badges)
- minimal: (no default text widgets)

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
| Layout | 4 (format_left, format_center, format_right, bar_bg) |
| Palette | 12 |
| Mode accent | 14 |
| mode widget | 18 (3 style + 1 format + 14 content) |
| session | 4 (3 style + 1 format) |
| tabs | 47 (6 style + 5 format/indicator + 4 separator + 32 sub-placeholder style/format) |
| cwd | 4 (3 style + 1 format) |
| command (per NAME) | 6 |
| text (per NAME) | 5 |
| tooltip | 10 |
| **Total** | **~117 + 6-11/user-defined widget** |

### Base mode detection

- Auto-detect from keybindings: count `SwitchToMode` transitions, the most frequent target is the home mode
- `base_mode "locked"` / `base_mode "normal"` for explicit override

### Theme presets

Themes provide: palette colors (12 colors including surface/surface_bright) + mode accent colors. Style presets provide: format strings + widget styles + default text widgets.

Available themes: system (default, uses zellij's palette), tokyonight, catppuccin-mocha, nord, gruvbox-dark.

System theme: `bg` = `ribbon_unselected.base` (maps to `palette.black` in zellij). Surface colors auto-computed via `lighten_color`.

### Style presets

4 built-in styles, each defining format strings, widget styles, and default text widgets:

- **simple** (default): Flat look with thin `|` separators and icons. All sections in format_left/format_right.
- **minimal**: Dotbar style — mode left, tabs centered (`format_center`), time right. Uses `tab_separator " • "` and lowercase mode text.
- **powerline**: Triangle arrow separators using positional color refs (`prev_bg`/`next_bg`). Tabs use `surface`/`surface_bright` backgrounds.
- **bubble**: Rounded pill segments with two-tone icon badges (accent bg icon + muted `surface` text area). Each widget floats as an isolated pill.

## TODO

### Tooltip

- [ ] Multi-column layout (future: `tooltip_columns "auto"`)

### Documentation

- [ ] Update `README.md` with current configuration spec and usage examples
- [ ] Update `../zellij-hud.wiki/` (Configuration.md, Architecture.md, etc.)

## Upstream proposals (zellij)

### `pipe_message_to_plugin` spawn target tab

`pipe_message_to_plugin` with `zellij:OWN_URL` always places the new pane on the **first connected client's active tab** (`cli_client_id = None` in the internal flow). This causes a brief flash on wrong tabs in multi-client scenarios, because the plugin must `break_panes_to_tab_with_index` after spawn to move it.

**Proposal**: Add a target tab field to `MessageToPlugin` (e.g. `new_plugin_instance_should_open_in_tab(tab_index)`) so plugins can specify the destination tab at spawn time, avoiding the post-spawn move and the flash.

### ~~Per-pane borderless floating panes~~ (resolved in v0.44.0)

Resolved: zellij 0.44.0 added `FloatingPaneCoordinates::borderless` and `set_pane_borderless()` API.
