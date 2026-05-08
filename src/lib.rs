//! Pure logic (no zellij-tile shim calls) shared between the bin and tests.
//!
//! Files that only `use zellij_tile::prelude::*` for *types* (Action,
//! InputMode, ModeInfo, KeyWithModifier, Styling, …) live here. Files that
//! actually call zellij-tile shim functions (close_self,
//! pipe_message_to_plugin, set_timeout, …) cannot be linked against the host
//! target and must stay in the bin.

pub mod action_types;
pub mod commands;
pub mod config;
pub mod keybinds;
pub mod spans;
pub mod text;
pub mod tooltip_layout;
