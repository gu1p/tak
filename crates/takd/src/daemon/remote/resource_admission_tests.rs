#![cfg(test)]

use super::test_support::request;
use super::{ResourceAdmissionDecision, ResourceCapacity, SharedResourceAdmission};

#[test]
fn external_memory_usage_does_not_block_resource_reservations() {
    let admission = SharedResourceAdmission::new_for_tests(ResourceCapacity {
        cpu_cores: 8.0,
        memory_mb: 1024,
    });

    let decision = admission
        .admit_or_queue(request("memory-heavy", 1.0, 400))
        .expect("admission decision");

    assert!(matches!(decision, ResourceAdmissionDecision::Admitted));
}

#[test]
fn external_cpu_usage_does_not_block_resource_reservations() {
    let admission = SharedResourceAdmission::new_for_tests(ResourceCapacity {
        cpu_cores: 8.0,
        memory_mb: 4096,
    });

    let decision = admission
        .admit_or_queue(request("cpu-heavy", 1.0, 512))
        .expect("admission decision");

    assert!(matches!(decision, ResourceAdmissionDecision::Admitted));
}

#[test]
fn reservations_queue_when_declared_tak_usage_exceeds_capacity() {
    let admission = SharedResourceAdmission::new_for_tests(ResourceCapacity {
        cpu_cores: 8.0,
        memory_mb: 4096,
    });
    let first = admission
        .admit_or_queue(request("running", 7.5, 3800))
        .expect("first admission");

    let decision = admission
        .admit_or_queue(request("next", 1.0, 512))
        .expect("admission decision");

    assert!(matches!(first, ResourceAdmissionDecision::Admitted));
    assert!(matches!(
        decision,
        ResourceAdmissionDecision::Queued { queue_position: 1 }
    ));
}
