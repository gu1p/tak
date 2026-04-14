use std::time::Duration;

use takd::agent::{TorRecoveryBackoff, TorRecoveryTracker};

#[test]
fn tor_recovery_tracker_requires_three_consecutive_failures_and_resets_after_success() {
    let mut tracker = TorRecoveryTracker::new(3);

    assert!(!tracker.record_failure());
    assert!(!tracker.record_failure());
    tracker.record_success();
    assert_eq!(tracker.consecutive_failures(), 0);

    assert!(!tracker.record_failure());
    assert!(!tracker.record_failure());
    assert!(tracker.record_failure());
    assert_eq!(tracker.consecutive_failures(), 3);
}

#[test]
fn tor_recovery_backoff_doubles_until_the_cap() {
    let mut backoff = TorRecoveryBackoff::new(Duration::from_secs(5), Duration::from_secs(60));

    assert_eq!(backoff.next_delay(), Duration::from_secs(5));
    assert_eq!(backoff.next_delay(), Duration::from_secs(10));
    assert_eq!(backoff.next_delay(), Duration::from_secs(20));
    assert_eq!(backoff.next_delay(), Duration::from_secs(40));
    assert_eq!(backoff.next_delay(), Duration::from_secs(60));
    assert_eq!(backoff.next_delay(), Duration::from_secs(60));

    backoff.reset();
    assert_eq!(backoff.next_delay(), Duration::from_secs(5));
}
