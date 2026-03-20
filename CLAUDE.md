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

Single WASM binary (`zellij-tile = "0.43.1"`), three roles distinguished by `Role` enum:

### Source files

- `src/main.rs` — Plugin state, `Role` enum, event handling, role dispatch
- `src/config.rs` — Configuration parsing, `ThemePalette`, theme presets, `IconColors`
- `src/render.rs` — HUD status bar rendering with `print_text()`/`Text` API
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
- Floating pane height must be 3+ (border lines consume 2 rows)
- `Text` API `color_range()` uses **character indices** (`.chars().count()`), not byte indices
- `print_text(text)` for theme-colored output with `Text::new().opaque()`

### Tab following

- `get_plugin_ids()` is synchronous — call in `load()`
- Use `tabs.iter().position(|t| t.active)` (iterator position, NOT `TabInfo.position` field)
- `break_panes_to_tab_with_index` moves HUD to active tab

### Timer

- `set_timeout(1.0)` + `Event::Timer(_)` for 1-second clock updates
- `std::time::SystemTime::now()` works in wasm32-wasip1

### Frame

- `rename_plugin_pane(id, "")` clears frame title
- No per-pane borderless API for floating panes — frames always visible
- `toggle_pane_frames()` is global only

### Permissions

- Daemon/HUD both need: `ReadApplicationState`, `ChangeApplicationState`, `MessageAndLaunchOtherPlugins`, `RunCommands`

## Theme Palette System

### Priority chain

```
color_* override > palette_* override > theme preset > tokyonight default
```

### Palette (10 colors)

`fg`, `bg`, `dim`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `orange`

### `color_*` values

Hex (`#RRGGBB`) or palette names (`"magenta"`, `"cyan"`, etc.)

### Theme presets

tokyonight (default), catppuccin-mocha, nord, gruvbox-dark

### Icon colors

`IconColors` struct with 8 category fields (navigation, create, close, resize, toggle, search, mode_switch, dim), derived from palette. Used by `ActionType::icon_color()`.

## Base Mode Detection

### Config

- `base_mode "auto"` (default) — auto-detect from keybindings
- `base_mode "locked"` / `base_mode "normal"` — explicit override

### Auto-detection logic

Count incoming `SwitchToMode` transitions: the mode other modes switch back to most frequently is the home mode.

### Enable/disable

- `enable_status_bar "false"` (default: true)
- `enable_tooltip "false"` (default: true)

## TODO

- [ ] Theme file splitting: split presets into `src/themes/tokyonight.rs` etc.
- [ ] Frame invisibility: find a way to hide floating pane border
- [ ] compact-bar replacement: once stable, disable built-in compact-bar
- [ ] Theme-aware colors: use `mode_info.style.colors` for dynamic color mapping
