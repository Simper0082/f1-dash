use std::collections::HashMap;

use serde_json::Value;

/// "2026-11-22T04:34:26.101Z" (fraction and trailing Z optional) -> seconds since the unix epoch.
pub fn iso_secs(s: &str) -> Option<f64> {
    let s = s.trim().trim_end_matches('Z');
    let (date, time) = s.split_once('T')?;
    let mut d = date.split('-');
    let (y, m, day): (i64, i64, i64) = (
        d.next()?.parse().ok()?,
        d.next()?.parse().ok()?,
        d.next()?.parse().ok()?,
    );
    let mut t = time.split(':');
    let (h, mi): (f64, f64) = (t.next()?.parse().ok()?, t.next()?.parse().ok()?);
    let sec: f64 = t.next()?.parse().ok()?;

    // days from civil (Howard Hinnant)
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe - 719468;

    Some(days as f64 * 86400.0 + h * 3600.0 + mi * 60.0 + sec)
}

#[derive(Debug, Clone, Default)]
pub struct PitRecord {
    pub in_at: Option<f64>,
    pub last_lane_s: Option<f64>,
    pub last_out_at: Option<f64>,
    pub stops: u32,
}

/// Remembers when each car entered and left the pit lane, using the feed's own timestamps so it
/// behaves the same live and in a replay. The pit-lane time is the InPit true -> false gap.
#[derive(Debug, Default)]
pub struct PitTracker {
    pub now: Option<f64>,
    pub drivers: HashMap<String, PitRecord>,
}

impl PitTracker {
    pub fn note(&mut self, topic: &str, data: &Value, ts: &str) {
        let Some(at) = iso_secs(ts) else { return };
        self.now = Some(at);

        if topic != "TimingData" {
            return;
        }
        let Some(lines) = data.get("Lines").and_then(|l| l.as_object()) else {
            return;
        };
        for (nr, line) in lines {
            let Some(in_pit) = line.get("InPit").and_then(|v| v.as_bool()) else {
                continue;
            };
            let rec = self.drivers.entry(nr.clone()).or_default();
            match (in_pit, rec.in_at) {
                (true, None) => rec.in_at = Some(at),
                (false, Some(start)) => {
                    rec.last_lane_s = Some(((at - start) * 10.0).round() / 10.0);
                    rec.last_out_at = Some(at);
                    rec.stops += 1;
                    rec.in_at = None;
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_iso() {
        assert_eq!(iso_secs("1970-01-02T00:00:01Z"), Some(86401.0));
        let a = iso_secs("2026-11-22T04:34:26.500Z").unwrap();
        let b = iso_secs("2026-11-22T04:34:27").unwrap();
        assert!((b - a - 0.5).abs() < 1e-6);
    }

    #[test]
    fn times_a_stop() {
        let mut t = PitTracker::default();
        let on = json!({"Lines": {"3": {"InPit": true}}});
        let off = json!({"Lines": {"3": {"InPit": false}}});
        t.note("TimingData", &on, "2026-11-22T04:00:00.000Z");
        assert!(t.drivers["3"].in_at.is_some());
        t.note("TimingData", &off, "2026-11-22T04:00:21.400Z");
        let r = &t.drivers["3"];
        assert_eq!(r.last_lane_s, Some(21.4));
        assert_eq!(r.stops, 1);
        assert!(r.in_at.is_none());
    }
}
