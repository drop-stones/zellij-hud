pub(crate) const CMD_CONTEXT_TZ: &str = "tz_detect";
pub(crate) const CMD_CONTEXT_MEM: &str = "mem_usage";
pub(crate) const MEM_UPDATE_INTERVAL: u32 = 5;

/// Parse `date +%z` output (e.g. "+0900", "-0500") into hours offset.
pub(crate) fn parse_date_tz(stdout: &[u8]) -> Option<i64> {
    let s = std::str::from_utf8(stdout).ok()?.trim();
    if s.len() < 5 {
        return None;
    }
    let sign: i64 = if s.starts_with('-') { -1 } else { 1 };
    let digits = &s[1..];
    let hours: i64 = digits[..2].parse().ok()?;
    let mins: i64 = digits[2..4].parse().ok()?;
    Some(sign * hours + if mins > 0 { sign } else { 0 })
}

/// Parse `free -b` output into (used_bytes, total_bytes).
/// Looks for the "Mem:" line and extracts total (col 1) and used (col 2).
pub(crate) fn parse_free(stdout: &[u8]) -> Option<(u64, u64)> {
    let s = std::str::from_utf8(stdout).ok()?;
    for line in s.lines() {
        if line.starts_with("Mem:") {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() >= 3 {
                let total: u64 = cols[1].parse().ok()?;
                let used: u64 = cols[2].parse().ok()?;
                return Some((used, total));
            }
        }
    }
    None
}
