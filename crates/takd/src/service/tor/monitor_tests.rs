#![cfg(test)]

use crate::agent::TorRecoveryTracker;

use super::{TorHealthEvent, TorHealthTransition, handle_health_event};

#[test]
fn transient_failures_keep_transport_ready_until_threshold() {
    let mut tracker = TorRecoveryTracker::new(3);

    assert_eq!(
        handle_health_event(
            &mut tracker,
            TorHealthEvent::Failure("rendezvous accept failed".to_string()),
        ),
        TorHealthTransition::KeepReady
    );
    assert_eq!(
        handle_health_event(
            &mut tracker,
            TorHealthEvent::Failure("rendezvous accept failed again".to_string()),
        ),
        TorHealthTransition::KeepReady
    );

    let transition = handle_health_event(
        &mut tracker,
        TorHealthEvent::Failure("rendezvous accept failed final".to_string()),
    );
    let TorHealthTransition::Recovering(reason) = transition else {
        panic!("third consecutive failure should recover, got {transition:?}");
    };
    assert!(reason.contains("rendezvous accept failed final"));
    assert!(reason.contains("3 consecutive transport failures"));
}

#[test]
fn success_resets_failures_and_marks_transport_ready() {
    let mut tracker = TorRecoveryTracker::new(3);

    assert_eq!(
        handle_health_event(
            &mut tracker,
            TorHealthEvent::Failure("rendezvous accept failed".to_string()),
        ),
        TorHealthTransition::KeepReady
    );
    assert_eq!(
        handle_health_event(&mut tracker, TorHealthEvent::ProbeSucceeded),
        TorHealthTransition::Ready
    );
    assert_eq!(tracker.consecutive_failures(), 0);
    assert_eq!(
        handle_health_event(
            &mut tracker,
            TorHealthEvent::Failure("rendezvous accept failed after recovery".to_string()),
        ),
        TorHealthTransition::KeepReady
    );
}
