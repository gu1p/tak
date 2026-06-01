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

#[test]
fn request_larger_than_total_capacity_is_rejected() {
    let admission = SharedResourceAdmission::new_for_tests(ResourceCapacity {
        cpu_cores: 8.0,
        memory_mb: 4096,
    });

    let decision = admission
        .admit_or_queue(request("too-large", 16.0, 512))
        .expect("admission decision");

    assert!(matches!(
        decision,
        ResourceAdmissionDecision::Rejected { .. }
    ));
    assert!(admission.queued_jobs().expect("queued jobs").is_empty());
}

#[test]
fn duplicate_queued_request_keeps_single_fifo_entry() {
    let admission = SharedResourceAdmission::new_for_tests(ResourceCapacity {
        cpu_cores: 8.0,
        memory_mb: 4096,
    });
    admission
        .admit_or_queue(request("running", 8.0, 4096))
        .expect("running admission");
    let first = admission
        .admit_or_queue(request("queued", 1.0, 512))
        .expect("first queue");
    let duplicate = admission
        .admit_or_queue(request("queued", 1.0, 512))
        .expect("duplicate queue");

    assert!(matches!(
        first,
        ResourceAdmissionDecision::Queued { queue_position: 1 }
    ));
    assert!(matches!(
        duplicate,
        ResourceAdmissionDecision::Queued { queue_position: 1 }
    ));
    assert_eq!(admission.queued_jobs().expect("queued jobs").len(), 1);
}
