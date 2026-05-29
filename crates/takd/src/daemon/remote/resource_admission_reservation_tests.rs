#![cfg(test)]

use super::test_support::request;
use super::{ResourceAdmissionDecision, ResourceCapacity, SharedResourceAdmission};

#[test]
fn active_tak_reservations_are_counted_at_declared_max() {
    let admission = SharedResourceAdmission::new_for_tests(ResourceCapacity {
        cpu_cores: 8.0,
        memory_mb: 4096,
    });

    let first = admission
        .admit_or_queue(request("already-running", 7.5, 3800))
        .expect("first admission");
    let second = admission
        .admit_or_queue(request("next", 1.0, 1024))
        .expect("second admission");

    assert!(matches!(first, ResourceAdmissionDecision::Admitted));
    assert!(matches!(
        second,
        ResourceAdmissionDecision::Queued { queue_position: 1 }
    ));
}
