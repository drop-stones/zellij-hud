# zellij-hud

On-demand floating status bar and which-key tooltip for [zellij](https://zellij.dev/).

Hidden in your base mode (zero footprint), appears as floating panes when you switch modes.

## Features

- **Floating status bar** — session name, mode indicator, tabs, CWD, date/time, memory usage
- **Which-key tooltip** — context-aware keybinding hints that auto-resize per mode
- **Theme presets** — tokyonight (default), catppuccin-mocha, nord, gruvbox-dark
- **Fully configurable** — colors, layout format, per-mode colors, enable/disable components
- **Base mode detection** — works with both locked-centric and normal-centric keybind setups

## Architecture

Single WASM binary, three roles:

1. **Daemon** — loaded by the user, hides itself. Spawns HUD and Tooltip panes on mode change.
2. **HUD** — floating status bar at the bottom. Closes itself when returning to base mode.
3. **Tooltip** — floating which-key hints at the bottom-right. Dynamically resizes on mode change.

## Requirements

- Zellij 0.43.1+
- [Nerd Fonts](https://www.nerdfonts.com/) (for icons)

## Installation

### Automated Installation

Zellij can download the plugin automatically when you specify it with its release URL. Add to your zellij config (`config.kdl`):

```kdl
load_plugins {
    "https://github.com/drop-stones/zellij-hud/releases/latest/download/zellij-hud.wasm" {
        theme "tokyonight"
    }
}
```

> **Note:** Zellij may have a bug that corrupts downloads when multiple tabs load the plugin simultaneously. If you experience issues, use manual installation instead.

### Manual Installation

Download the latest `zellij-hud.wasm` from [GitHub Releases](https://github.com/drop-stones/zellij-hud/releases/latest) and place it somewhere zellij can access. Then add to your zellij config (`config.kdl`):

```kdl
load_plugins {
    "file:/path/to/zellij-hud.wasm" {
        theme "tokyonight"
    }
}
```

### Nix Flakes

Add this repository to your flake inputs:

```nix
inputs = {
  zellij-hud.url = "github:drop-stones/zellij-hud";
};
```

Then expose the package via an overlay:

```nix
overlays = [
  (final: prev: {
    zellij-hud = inputs.zellij-hud.packages.${prev.system}.default;
  })
];
```

You can then reference the plugin as `${pkgs.zellij-hud}/bin/zellij-hud.wasm`. With home-manager:

```nix
xdg.configFile."zellij/config.kdl".text = ''
  load_plugins {
      "file:${pkgs.zellij-hud}/bin/zellij-hud.wasm" {
          theme "tokyonight"
      }
  }
'';
```

> **Note:** Load via `load_plugins` or `zellij plugin -- file:/path/to/zellij-hud.wasm`. Loading via layout may cause issues.

## Configuration

All settings are optional. Place them inside the plugin block:

```kdl
load_plugins {
    "file:/path/to/zellij-hud.wasm" {
        theme "catppuccin-mocha"
        format_left "{mode} | {session} | {tabs}"
        format_right "{cwd} | {date} | {time}"
        enable_status_bar true
        enable_tooltip true
        base_mode "auto"
    }
}
```

### General

| Key | Default | Description |
|-----|---------|-------------|
| `theme` | `"tokyonight"` | Theme preset (`tokyonight`, `catppuccin-mocha`, `nord`, `gruvbox-dark`) |
| `format_left` | `"{session} \| {mode} \| {tabs}"` | Left side format string |
| `format_right` | `"{cwd} \| {memory} \| {date} \| {time}"` | Right side format string |
| `separator` | `"│"` | Separator character between segments |
| `timezone` | `0` | Timezone offset in hours (auto-detected at startup) |
| `enable_status_bar` | `true` | Show/hide the floating status bar |
| `enable_tooltip` | `true` | Show/hide the which-key tooltip |
| `base_mode` | `"auto"` | Base mode detection: `"auto"`, `"locked"`, or `"normal"` |

### Format Placeholders

| Placeholder | Description |
|-------------|-------------|
| `{session}` | Session name |
| `{mode}` | Current input mode with icon |
| `{tabs}` | Tab list (active tab highlighted) |
| `{cwd}` | Current working directory (basename) |
| `{date}` | Current date |
| `{time}` | Current time |
| `{memory}` | Memory usage percentage |

### Color Overrides

Colors accept hex values (`#RRGGBB`) or palette names (`fg`, `dim`, `red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `orange`).

#### Priority

```
color_* override > palette_* override > theme preset > tokyonight default
```

#### Status bar colors

| Key | Default (palette) |
|-----|-------------------|
| `color_session` | `cyan` |
| `color_mode` | `blue` |
| `color_tab_active` | `fg` |
| `color_tab_inactive` | `dim` |
| `color_cwd` | `cyan` |
| `color_date` | `magenta` |
| `color_time` | `blue` |
| `color_memory` | `green` |
| `color_separator` | `dim` |

#### Tooltip colors

| Key | Default (palette) |
|-----|-------------------|
| `color_tooltip_key` | `cyan` |
| `color_tooltip_arrow` | `dim` |
| `color_tooltip_action` | `magenta` |
| `color_tooltip_mode` | `blue` |

#### Per-mode colors

| Key | Default (palette) |
|-----|-------------------|
| `color_mode_normal` | `green` |
| `color_mode_locked` | `dim` |
| `color_mode_pane` | `orange` |
| `color_mode_tab` | `yellow` |
| `color_mode_resize` | `red` |
| `color_mode_move` | `magenta` |
| `color_mode_scroll` | `cyan` |
| `color_mode_session` | `magenta` |
| `color_mode_search` | `yellow` |
| `color_mode_rename_tab` | `yellow` |
| `color_mode_rename_pane` | `yellow` |
| `color_mode_enter_search` | `yellow` |
| `color_mode_tmux` | `orange` |
| `color_mode_prompt` | `blue` |

### Palette Overrides

Override individual palette colors without switching themes:

```kdl
load_plugins {
    "file:/path/to/zellij-hud.wasm" {
        theme "tokyonight"
        palette_orange "#e0af68"
        palette_dim "#3b4261"
    }
}
```

Available keys: `palette_fg`, `palette_dim`, `palette_red`, `palette_green`, `palette_yellow`, `palette_blue`, `palette_magenta`, `palette_cyan`, `palette_orange`.

### Base Mode

The base mode determines when the HUD hides. Set to `"auto"` (default) to use zellij's `default_mode` setting, or override explicitly:

```kdl
base_mode "locked"   // for locked-centric keybind setups
base_mode "normal"   // for normal-centric keybind setups
base_mode "auto"     // use zellij's default_mode (default)
```

## Theme Presets

- `tokyonight` (default)
- `catppuccin-mocha`
- `nord`
- `gruvbox-dark`

## License

MIT
