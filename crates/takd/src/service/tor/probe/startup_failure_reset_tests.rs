use super::{StartupTorFailureDecision, StartupTorFailureTracker};

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
