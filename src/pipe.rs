//! Helpers for the inter-instance pipe protocol.
//!
//! The daemon and HUD/Tooltip instances communicate via `pipe_message_to_plugin`
//! using string payloads. These pure parsers live here so the bin-side handlers
//! stay focused on side effects.

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
