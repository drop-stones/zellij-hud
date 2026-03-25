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
- HUD is borderless (height=1); tooltip keeps frame (height = content + 2 border rows)
- `Text` API `color_range()` uses **character indices** (`.chars().count()`), not byte indices
- `print_text(text)` for theme-colored output with `Text::new().opaque()`
- HUD background color: `\x1b[0m` resets in segment output are replaced with `\x1b[0m{bg}` to maintain background across segments

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

## Theme Palette System

### Priority chain

```
color_* override > palette_* override > theme preset > tokyonight default
```

### Palette (10 colors)

`fg`, `bg`, `dim`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `orange`

- `bg` is used for HUD status bar background color
- System theme: `bg` = `ribbon_unselected.base` (maps to `palette.black` in zellij)

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

- [ ] Powerline-style segments: render HUD segments with powerline arrow separators (e.g. ` NORMAL   main `) using background/foreground color transitions per segment
- [ ] Theme file splitting: split presets into `src/themes/tokyonight.rs` etc.
- [ ] compact-bar replacement: once stable, disable built-in compact-bar
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
