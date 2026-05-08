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
