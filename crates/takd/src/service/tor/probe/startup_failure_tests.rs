#![cfg(test)]

use std::time::Duration;

use super::startup_probe_error;

#[test]
fn final_probe_timeout_preserves_earlier_tor_failure_signal() {
    let err = startup_probe_error(
        anyhow::anyhow!(
            "connect takd hidden-service startup probe timed out before the attempt started"
        ),
        Some(
            "connect takd hidden-service startup probe: Unable to select a guard relay: \
             No usable guards. Rejected 60/60 as down",
        ),
        "http://builder-a.onion",
        Duration::from_secs(60),
    );

    let detail = format!("{err:#}");
    assert!(detail.contains("did not become reachable within 60000ms during takd startup"));
    assert!(detail.contains("earlier Tor startup probe failure"));
    assert!(detail.contains("No usable guards"));
}
