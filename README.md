# 🪟 zellij-hud

On-demand floating status bar and which-key tooltip for [zellij](https://zellij.dev/).

Hidden in your base mode (zero footprint), appears as floating panes when you switch modes.

![demo](https://raw.githubusercontent.com/wiki/drop-stones/zellij-hud/demo/demo.gif)

## Features

- **Floating status bar** — session name, mode indicator, tabs, CWD, date/time, memory usage
- **Which-key tooltip** — context-aware keybinding hints that auto-resize per mode
- **Theme presets** — tokyonight (default), catppuccin-mocha, nord, gruvbox-dark
- **Fully configurable** — colors, layout format, per-mode colors, enable/disable components
- **Base mode detection** — works with both locked-centric and normal-centric keybind setups

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

You can then reference the plugin directly via `inputs.zellij-hud.packages.${pkgs.system}.default`. With home-manager:

```nix
xdg.configFile."zellij/config.kdl".text = ''
  load_plugins {
      "file:${inputs.zellij-hud.packages.${pkgs.system}.default}/bin/zellij-hud.wasm" {
          theme "tokyonight"
      }
  }
'';
```

> **Note:** Load via `load_plugins` or `zellij plugin -- file:/path/to/zellij-hud.wasm`. Loading via layout may cause issues.

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
