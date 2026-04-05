# ◨ zellij-hud

On-demand floating status bar and which-key tooltip for [zellij](https://zellij.dev/).

Hidden in your base mode (zero footprint), appears as floating panes when you switch modes.

![demo](https://raw.githubusercontent.com/wiki/drop-stones/zellij-hud/demo/demo.gif)

**simple** (default)

![simple](https://raw.githubusercontent.com/wiki/drop-stones/zellij-hud/screenshots/simple.png)

**minimal**

![minimal](https://raw.githubusercontent.com/wiki/drop-stones/zellij-hud/screenshots/minimal.png)

**powerline**

![powerline](https://raw.githubusercontent.com/wiki/drop-stones/zellij-hud/screenshots/powerline.png)

**bubble**

![bubble](https://raw.githubusercontent.com/wiki/drop-stones/zellij-hud/screenshots/bubble.png)

## Features

- **Floating status bar** — session name, mode indicator, tabs, CWD, git branch, date/time, memory usage
- **Which-key tooltip** — context-aware keybinding hints that auto-resize per mode
- **4 style presets** — simple, minimal, powerline, bubble — or build your own with composable widgets
- **5 theme presets** — system (auto), tokyonight, catppuccin-mocha, nord, gruvbox-dark
- **Tab indicators** — fullscreen and sync-panes status per tab with configurable styles
- **Composable widgets** — define custom command or text widgets and compose them in format strings
- **Fully configurable** — 12-color palette, per-mode accent colors, layout format strings, and 100+ config keys
- **Base mode detection** — works with both locked-centric and normal-centric keybind setups

## Requirements

- Zellij 0.44.0+
- [Nerd Fonts](https://www.nerdfonts.com/) (for icons)

## Installation

Add the following to your zellij config (`config.kdl`):

```kdl
plugins {
    // You can also use a local path: location="file:/path/to/zellij-hud.wasm"
    zellij-hud location="https://github.com/drop-stones/zellij-hud/releases/latest/download/zellij-hud.wasm"
}

load_plugins {
    "zellij-hud"
}
```

`plugins` defines the plugin alias with its location and configuration. `load_plugins` loads it on startup. Zellij downloads the plugin on first use and caches it.

## Permissions

On first load, zellij will prompt you to grant the following permissions:

| Permission | Reason |
|---|---|
| ReadApplicationState | Subscribe to mode changes and tab updates |
| ChangeApplicationState | Manage floating panes (spawn, close, resize, move across tabs) |
| MessageAndLaunchOtherPlugins | Spawn HUD and Tooltip pane instances |
| RunCommands | Execute command widgets (`date`, `free`, `git branch`, etc.) |

## Configuration

### Quick start

```kdl
plugins {
    zellij-hud location="https://github.com/drop-stones/zellij-hud/releases/latest/download/zellij-hud.wasm" {
        style "powerline"          // "simple" (default) | "minimal" | "powerline" | "bubble"
        theme "catppuccin-mocha"   // "system" (default) | "tokyonight" | "catppuccin-mocha" | "nord" | "gruvbox-dark"
    }
}

load_plugins {
    "zellij-hud"
}
```

See [`examples/`](examples/) for fully explicit config files for each style.

### Further reading

See the [wiki](https://github.com/drop-stones/zellij-hud/wiki) for detailed documentation:

- [Configuration](https://github.com/drop-stones/zellij-hud/wiki/Configuration) — global settings overview
  - [Colors](https://github.com/drop-stones/zellij-hud/wiki/Colors) — color system ([Color Values](https://github.com/drop-stones/zellij-hud/wiki/Color-Values), [Themes](https://github.com/drop-stones/zellij-hud/wiki/Themes))
  - [Status Bar](https://github.com/drop-stones/zellij-hud/wiki/Status-Bar) — [Styles](https://github.com/drop-stones/zellij-hud/wiki/Styles), [Widget Types](https://github.com/drop-stones/zellij-hud/wiki/Widget-Types), [Built-in Widgets](https://github.com/drop-stones/zellij-hud/wiki/Built-in-Widgets), [Custom Widgets](https://github.com/drop-stones/zellij-hud/wiki/Custom-Widgets)
  - [Tooltip](https://github.com/drop-stones/zellij-hud/wiki/Tooltip) — which-key tooltip settings
- [Architecture](https://github.com/drop-stones/zellij-hud/wiki/Architecture) — three-role design and permissions

## Acknowledgements

- [zellij compact-bar](https://github.com/zellij-org/zellij/tree/main/zellij-utils/assets/plugins/compact-bar) — reference implementation for status bar rendering as a zellij plugin
- [which-key.nvim](https://github.com/folke/which-key.nvim) — inspiration for the which-key tooltip appearance

## License

MIT
