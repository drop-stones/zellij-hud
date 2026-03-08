use std::time::{SystemTime, UNIX_EPOCH};

use crate::State;

impl State {
    pub(crate) fn format_time(&self) -> String {
        if let Ok(dur) = SystemTime::now().duration_since(UNIX_EPOCH) {
            let total_secs = dur.as_secs();
            let offset = self.hud_config.timezone_offset;
            let adjusted = (total_secs as i64 + offset * 3600).rem_euclid(86400);
            let hours = adjusted / 3600;
            let mins = (adjusted % 3600) / 60;
            format!("{:02}:{:02}", hours, mins)
        } else {
            "--:--".to_string()
        }
    }

    pub(crate) fn format_date(&self) -> String {
        if let Ok(dur) = SystemTime::now().duration_since(UNIX_EPOCH) {
            let offset = self.hud_config.timezone_offset;
            let adjusted_secs = dur.as_secs() as i64 + offset * 3600;
            let days = adjusted_secs.div_euclid(86400);

            let (_year, month, day) = days_to_ymd(days);
            let month_name = match month {
                1 => "Jan",
                2 => "Feb",
                3 => "Mar",
                4 => "Apr",
                5 => "May",
                6 => "Jun",
                7 => "Jul",
                8 => "Aug",
                9 => "Sep",
                10 => "Oct",
                11 => "Nov",
                12 => "Dec",
                _ => "???",
            };
            format!("{} {:02}", month_name, day)
        } else {
            "--- --".to_string()
        }
    }
}

pub(crate) fn days_to_ymd(days_since_epoch: i64) -> (i64, u32, u32) {
    let z = days_since_epoch + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}
