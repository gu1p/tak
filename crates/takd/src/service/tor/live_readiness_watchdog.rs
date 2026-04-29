use std::time::Duration;

use tor_hsservice::status::State as OnionServiceState;

pub(super) fn startup_watchdog_detail(
    service_state: OnionServiceState,
    startup_timeout: Duration,
) -> String {
    format!(
        "Arti onion-service state={service_state:?} did not become probeable within {}ms during takd startup",
        startup_timeout.as_millis()
    )
}

pub(super) fn startup_watchdog_restart_reason(detail: &str) -> String {
    format!("embedded Arti client startup watchdog expired: {detail}")
}
