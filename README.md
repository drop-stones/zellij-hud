# ◨ zellij-hud

On-demand floating status bar and which-key tooltip for [zellij](https://zellij.dev/).

Hidden in your base mode (zero footprint), appears as floating panes when you switch modes.

![demo](https://raw.githubusercontent.com/wiki/drop-stones/zellij-hud/demo/demo.gif)

## Features

- **Floating status bar** — session name, mode indicator, tabs, CWD, date/time, memory usage
- **Which-key tooltip** — context-aware keybinding hints that auto-resize per mode
- **Auto theme** — inherits colors from your zellij theme automatically
- **Theme presets** — tokyonight, catppuccin-mocha, nord, gruvbox-dark (manual override)
- **Fully configurable** — colors, layout format, per-mode colors, enable/disable components
- **Base mode detection** — works with both locked-centric and normal-centric keybind setups

## Requirements

- Zellij 0.43.1+
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
| RunCommands | Run `date` (timezone detection) and `free` (memory usage) |

## Configuration

See the [wiki](https://github.com/drop-stones/zellij-hud/wiki) for detailed documentation:

- [Configuration](https://github.com/drop-stones/zellij-hud/wiki/Configuration) — all settings, format placeholders, color and palette overrides
- [Themes](https://github.com/drop-stones/zellij-hud/wiki/Themes) — theme presets and customization
- [Architecture](https://github.com/drop-stones/zellij-hud/wiki/Architecture) — plugin internals

## Acknowledgements

- [zellij compact-bar](https://github.com/zellij-org/zellij/tree/main/zellij-utils/assets/plugins/compact-bar) — reference implementation for status bar rendering as a zellij plugin
- [which-key.nvim](https://github.com/folke/which-key.nvim) — inspiration for the which-key tooltip appearance

## License

MIT
