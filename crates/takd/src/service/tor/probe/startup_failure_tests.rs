#![cfg(test)]

use std::time::Duration;

use super::{StartupTorFailureDecision, StartupTorFailureTracker, startup_probe_error};

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

#[test]
fn descriptor_download_failures_restart_tor_client_after_threshold() {
    let mut tracker = StartupTorFailureTracker::new(3);
    let detail = "connect takd hidden-service startup probe: tor: error connecting to Tor: \
                  Unable to download hidden service descriptor";

    assert_eq!(
        tracker.record_failure(detail),
        StartupTorFailureDecision::KeepWaiting
    );
    assert_eq!(
        tracker.record_failure(detail),
        StartupTorFailureDecision::KeepWaiting
    );

    let StartupTorFailureDecision::RestartTorClient { reason } = tracker.record_failure(detail)
    else {
        panic!("third consecutive descriptor failure should restart the Tor client");
    };
    assert!(reason.contains("3 consecutive Tor startup probe failures"));
    assert!(reason.contains("Unable to download hidden service descriptor"));
}

#[test]
fn hidden_service_circuit_failures_restart_tor_client_after_threshold() {
    let mut tracker = StartupTorFailureTracker::new(2);
    let detail = "connect takd hidden-service startup probe: Failed to obtain hidden service \
                  circuit to builder.onion";

    assert_eq!(
        tracker.record_failure(detail),
        StartupTorFailureDecision::KeepWaiting
    );

    let StartupTorFailureDecision::RestartTorClient { reason } = tracker.record_failure(detail)
    else {
        panic!("second consecutive hidden-service circuit failure should restart the Tor client");
    };
    assert!(reason.contains("2 consecutive Tor startup probe failures"));
    assert!(reason.contains("Failed to obtain hidden service circuit"));
}

#[test]
fn guard_exhaustion_restarts_tor_client_without_waiting_for_threshold() {
    let mut tracker = StartupTorFailureTracker::new(3);

    let StartupTorFailureDecision::RestartTorClient { reason } = tracker.record_failure(
        "connect takd hidden-service startup probe: Unable to select a guard relay: \
         No usable guards. Rejected 60/60 as down",
    ) else {
        panic!("guard exhaustion should restart the Tor client immediately");
    };
    assert!(reason.contains("Unable to select a guard relay"));
    assert!(reason.contains("No usable guards"));
}

#[test]
fn non_tor_probe_failures_reset_consecutive_startup_failure_count() {
    let mut tracker = StartupTorFailureTracker::new(2);
    let tor_detail = "connect takd hidden-service startup probe: \
                      Unable to download hidden service descriptor";

    assert_eq!(
        tracker.record_failure(tor_detail),
        StartupTorFailureDecision::KeepWaiting
    );
    assert_eq!(
        tracker.record_failure("node probe failed with HTTP 500"),
        StartupTorFailureDecision::KeepWaiting
    );
    assert_eq!(
        tracker.record_failure(tor_detail),
        StartupTorFailureDecision::KeepWaiting
    );

    let StartupTorFailureDecision::RestartTorClient { reason } = tracker.record_failure(tor_detail)
    else {
        panic!("second consecutive Tor failure after reset should restart the Tor client");
    };
    assert!(reason.contains("2 consecutive Tor startup probe failures"));
}
