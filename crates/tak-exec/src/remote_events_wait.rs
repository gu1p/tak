use std::env;
use std::time::Duration;

pub(crate) fn remote_events_max_wait_duration() -> Duration {
    parse_remote_events_max_wait_duration(
        env::var("TAK_REMOTE_EVENTS_MAX_WAIT_SECS").ok().as_deref(),
    )
}

pub(crate) fn parse_remote_events_max_wait_duration(raw: Option<&str>) -> Duration {
    let Some(raw) = raw else {
        return Duration::from_secs(120);
    };
    let raw = raw.trim();
    if raw.is_empty() {
        return Duration::from_secs(120);
    }
    match raw.parse::<u64>() {
        Ok(seconds) if seconds > 0 => Duration::from_secs(seconds),
        _ => Duration::from_secs(120),
    }
}
