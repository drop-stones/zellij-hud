//! Helpers for the inter-instance pipe protocol.
//!
//! The daemon and HUD/Tooltip instances communicate via `pipe_message_to_plugin`
//! using string payloads. These pure parsers live here so the bin-side handlers
//! stay focused on side effects.

use zellij_tile::prelude::InputMode;

use crate::input_mode::mode_from_str;

/// Parse a `"client_id:seq"` close pipe payload.
///
/// The daemon tags every spawn with the spawning client_id and a per-spawn
/// sequence number. The HUD/Tooltip checks both before honouring a close
/// request, so stale "close" pipes from previous spawn cycles are ignored.
///
/// Returns `None` if the payload is malformed (missing colon, non-numeric
/// fields, or out-of-range integers).
pub fn parse_close_payload(payload: &str) -> Option<(u16, u32)> {
    let (cid_str, seq_str) = payload.split_once(':')?;
    let cid: u16 = cid_str.parse().ok()?;
    let seq: u32 = seq_str.parse().ok()?;
    Some((cid, seq))
}

/// Parse a `"client_id:rest"` payload — the common envelope for several pipe
/// messages (mode_info_sync, cwd_update). Returns the parsed client_id and a
/// borrowed slice of the remainder, leaving payload-specific parsing to the
/// caller (e.g. JSON for mode_info_sync, raw path for cwd_update).
pub fn parse_client_prefixed(payload: &str) -> Option<(u16, &str)> {
    let (id_str, rest) = payload.split_once(':')?;
    let cid: u16 = id_str.parse().ok()?;
    Some((cid, rest))
}

/// Parse a `"client_id:Mode"` mode_sync pipe payload, where `Mode` is the
/// `InputMode` Debug name (e.g. `"Normal"`, `"Pane"`).
///
/// Returns `None` if the envelope is malformed or the mode name is unknown.
pub fn parse_mode_sync_payload(payload: &str) -> Option<(u16, InputMode)> {
    let (cid, mode_str) = parse_client_prefixed(payload)?;
    let mode = mode_from_str(mode_str)?;
    Some((cid, mode))
}

/// Parse a `"client_id:name:value"` cmd_update pipe payload.
///
/// `value` is everything after the second `:`, so colons inside the command
/// output are preserved (`splitn(3, ':')`).
pub fn parse_cmd_update_payload(payload: &str) -> Option<(u16, &str, &str)> {
    let mut parts = payload.splitn(3, ':');
    let cid_str = parts.next()?;
    let name = parts.next()?;
    let value = parts.next()?;
    let cid: u16 = cid_str.parse().ok()?;
    Some((cid, name, value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_close_payload_accepts_well_formed_pairs() {
        assert_eq!(parse_close_payload("0:0"), Some((0, 0)));
        assert_eq!(parse_close_payload("12:34"), Some((12, 34)));
        // Boundary values for the chosen integer types.
        assert_eq!(
            parse_close_payload(&format!("{}:{}", u16::MAX, u32::MAX)),
            Some((u16::MAX, u32::MAX)),
        );
    }

    #[test]
    fn parse_close_payload_rejects_missing_separator() {
        assert_eq!(parse_close_payload(""), None);
        assert_eq!(parse_close_payload("12"), None);
        assert_eq!(parse_close_payload("12-34"), None);
    }

    #[test]
    fn parse_close_payload_rejects_non_numeric_fields() {
        assert_eq!(parse_close_payload("abc:1"), None);
        assert_eq!(parse_close_payload("1:abc"), None);
        assert_eq!(parse_close_payload(":1"), None);
        assert_eq!(parse_close_payload("1:"), None);
    }

    #[test]
    fn parse_close_payload_rejects_out_of_range_integers() {
        // u16 client_id overflows past 65535.
        assert_eq!(parse_close_payload("65536:0"), None);
        // u32 seq overflows past 2^32 - 1.
        assert_eq!(
            parse_close_payload(&format!("0:{}", u64::from(u32::MAX) + 1)),
            None,
        );
        // Negative numbers are rejected because both fields are unsigned.
        assert_eq!(parse_close_payload("-1:0"), None);
    }
}
