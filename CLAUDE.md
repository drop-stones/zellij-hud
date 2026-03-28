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
- `src/config.rs` — Configuration parsing, `Color` type, `BarStyle`/`SeparatorPreset` enums, `ThemePalette`, theme presets, `IconColors`
- `src/render.rs` — HUD status bar rendering (minimal + powerline modes)
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
- Two bar styles: `BarStyle::Minimal` (flat bg + thin separators) and `BarStyle::Powerline` (per-segment bg + arrow separators)
- `SeparatorPreset` enum: `Triangle`, `Circle`, `Slant`, `Flame` — each defines minimal (thin) and powerline (thick) separator chars
- HUD background color: in minimal mode, `\x1b[0m` resets are replaced with `\x1b[0m{bg}` to maintain background across segments

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

## Configuration Spec (v3)

Design goals: zjstatus-level flexibility + on-demand display + tooltip = superset of zjstatus.
Key differentiators from zjstatus: mode accent color (global mode-reactive styling), declarative style keys (no inline `#[...]` syntax), automatic separator color calculation, predefined themes with zero-config defaults.

### Global

```kdl
theme "system"              // "system" (default) | "tokyonight" | "catppuccin-mocha" | "nord" | "gruvbox-dark"
enable_status_bar "true"
enable_tooltip "true"
```

### Layout

```kdl
format_left "{mode} {session} {tabs}"
format_right "{command_git_branch} {command_datetime}"
```

- Spaces between widgets are ignored (for readability only)
- Widget ordering is determined by format string
- Separation between widgets is controlled by each widget's `_separator`

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

Color values: palette name (`"blue"`, `"accent"`) or hex (`"#7aa2f7"`).

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

All widgets share these style keys:

```
{widget}_fg "{color}"          // foreground: palette name | hex | "accent"
{widget}_bg "{color}"          // background: same. If set, segment gets its own bg
{widget}_attr "{decorations}"  // "bold" | "italic" | "bold,italic" | ""
{widget}_separator "{char}"    // separator character after this widget (color auto-calculated from adjacent bg)
```

When bg is set on a widget, powerline-style rendering is automatic (separator color transitions based on adjacent widgets' bg). No `bar_style` toggle needed.

### Built-in widgets

#### mode

```kdl
mode_fg "bg"
mode_bg "accent"
mode_attr "bold"
mode_separator ""

mode_normal "󰍀 NORMAL"
mode_locked "󰌾 LOCKED"
mode_pane "󰘖 PANE"
mode_tab "󰓩 TAB"
mode_resize "󰩨 RESIZE"
mode_move "󰆾 MOVE"
mode_scroll "󰠶 SCROLL"
mode_search "󰍉 SEARCH"
mode_enter_search "󰍉 SEARCH"
mode_rename_tab "󰏫 RENAME TAB"
mode_rename_pane "󰏫 RENAME PANE"
mode_session "󱂬 SESSION"
mode_prompt "󰘥 PROMPT"
mode_tmux "󰰣 TMUX"
```

#### session

```kdl
session_fg "cyan"
session_bg ""
session_separator ""
session_format "󰆍 {name}"
```

Placeholders: `{name}`

#### tabs

```kdl
tab_active_fg "white"
tab_active_bg "blue"
tab_active_attr "bold"
tab_inactive_fg "dim"
tab_inactive_bg ""
tab_format "{name}"
tab_divider " "
tab_sync_indicator "🔗"
tab_fullscreen_indicator "⛶"
tabs_separator ""
```

Placeholders: `{name}`, `{index}`, `{sync_indicator}`, `{fullscreen_indicator}`

`tab_divider`: separator character between individual tabs (color auto-calculated from adjacent tabs' bg, same as widget separator).

### User-defined widgets

#### command (shell command execution)

```kdl
command_NAME_command "..."
command_NAME_fg "..."
command_NAME_bg ""
command_NAME_attr ""
command_NAME_separator ""
command_NAME_format "{stdout}"
command_NAME_interval "10"
```

Placeholders: `{stdout}`, `{stderr}`, `{exit_code}`

Referenced in format strings as `{command_NAME}`.

Example — datetime and git branch as commands:
```kdl
command_datetime_command "date +%H:%M"
command_datetime_fg "yellow"
command_datetime_format "󰥔 {stdout}"
command_datetime_interval "1"

command_git_branch_command "git rev-parse --abbrev-ref HEAD"
command_git_branch_fg "magenta"
command_git_branch_format " {stdout}"
command_git_branch_interval "10"
```

#### text (static text)

```kdl
text_NAME_content "🚀 dev"
text_NAME_fg "cyan"
text_NAME_bg ""
text_NAME_separator ""
```

Referenced as `{text_NAME}`.

### Tooltip

Tooltip uses `_color` (not `_fg`) because tooltip elements are text on a shared background — no per-element bg.

```kdl
// Colors (6 keys)
tooltip_key_color "cyan"              // keybinding key text
tooltip_separator_color "dim"         // separator between key and description
tooltip_description_color "fg"        // action description text
tooltip_mode_color "accent"           // description for mode-switch actions
tooltip_bg ""                         // tooltip content background
tooltip_border_color "dim"            // frame border color (via set_pane_color API)

// Display (4 keys)
tooltip_separator "➜"                 // character between key and description
tooltip_position "bottom-right"       // "bottom-right" | "bottom-left" | "top-right" | "top-left"
tooltip_title "{mode}"                // frame title. empty = no title
tooltip_border "true"                 // true | false (borderless via v0.44.0 API)
```

Border control uses `FloatingPaneCoordinates::borderless` at spawn time and `set_pane_color()` for frame color.

### Key count

| Category | Keys |
|---|---|
| Global | 3 |
| Layout | 2 |
| Palette | 10 |
| Mode accent | 13 |
| mode widget | 17 (4 style + 13 content) |
| session | 4 |
| tabs | 10 |
| command (per NAME) | 7 |
| text (per NAME) | 4 |
| tooltip | 10 |
| **Total** | **~69 + 11/user-defined** |

Most keys have sensible defaults from theme. Users typically write only a few.

### Base mode detection

- Auto-detect from keybindings: count `SwitchToMode` transitions, the most frequent target is the home mode
- `base_mode "locked"` / `base_mode "normal"` for explicit override

### Theme presets

Themes provide: palette colors + default widget styles (fg/bg/attr/separator for all widgets) + mode accent colors.

Available: system (default, uses zellij's palette), tokyonight, catppuccin-mocha, nord, gruvbox-dark.

System theme: `bg` = `ribbon_unselected.base` (maps to `palette.black` in zellij).

## TODO

### Status bar customization

- [ ] Implement config spec v3 (widget style keys, accent color, command/text widgets)
- [ ] Predefined theme configs (provide good defaults for powerline and minimal styles)

### Tooltip customization

- [ ] Implement tooltip config spec (colors, separator, position, border, title)
- [ ] Multi-column layout (future: `tooltip_columns "auto"`)

### Done

- [x] Powerline-style segments: `BarStyle::Powerline` with per-segment bg colors, arrow separators, and `SeparatorPreset` enum
- [x] Named separator presets: `Triangle`, `Circle`, `Slant`, `Flame` with minimal/powerline variants
- [x] Color type refactor: unified `Color` enum with `.fg()`/`.bg()` methods, replacing raw ANSI strings
- [x] HUD background color: added `bg` palette color and `color_bg` for solid status bar background
- [x] Borderless HUD: use `FloatingPaneCoordinates::borderless` for single-line HUD (zellij 0.44.0)
- [x] zellij 0.44.0 migration: updated Action enum to struct variants, zellij-tile 0.44.0
- [x] Theme-aware colors: use `mode_info.style.colors` for dynamic color mapping

## Upstream proposals (zellij)

### `pipe_message_to_plugin` spawn target tab

`pipe_message_to_plugin` with `zellij:OWN_URL` always places the new pane on the **first connected client's active tab** (`cli_client_id = None` in the internal flow). This causes a brief flash on wrong tabs in multi-client scenarios, because the plugin must `break_panes_to_tab_with_index` after spawn to move it.

**Proposal**: Add a target tab field to `MessageToPlugin` (e.g. `new_plugin_instance_should_open_in_tab(tab_index)`) so plugins can specify the destination tab at spawn time, avoiding the post-spawn move and the flash.

### ~~Per-pane borderless floating panes~~ (resolved in v0.44.0)

Resolved: zellij 0.44.0 added `FloatingPaneCoordinates::borderless` and `set_pane_borderless()` API.
