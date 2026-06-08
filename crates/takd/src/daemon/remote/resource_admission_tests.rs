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
fn tolerant_admission_admits_above_cumulative_capacity() {
    // With oversubscription, summed reservations may far exceed raw capacity:
    // the memory-pressure controller is the runtime backstop, not rejection.
    let admission = SharedResourceAdmission::new_for_tests_with_oversubscribe(
        ResourceCapacity {
            cpu_cores: 4.0,
            memory_mb: 4096,
        },
        16,
    );
    for index in 0..20 {
        let decision = admission
            .admit_or_queue(request(&format!("task-{index}"), 0.1, 512))
            .expect("admission decision");
        assert!(
            matches!(decision, ResourceAdmissionDecision::Admitted),
            "task {index} should be admitted under oversubscription: {decision:?}"
        );
    }
    // 20 * 512 MB = 10240 MB, well over the 4096 MB raw capacity, none queued.
    assert!(admission.queued_jobs().expect("queued jobs").is_empty());
}

#[test]
fn emergency_hold_queues_new_starts_until_cleared() {
    let admission = SharedResourceAdmission::new_for_tests_with_oversubscribe(
        ResourceCapacity {
            cpu_cores: 8.0,
            memory_mb: 8192,
        },
        16,
    );
    admission.set_admission_held(true).expect("hold");
    let held = admission
        .admit_or_queue(request("start", 1.0, 512))
        .expect("admission decision");
    assert!(
        matches!(held, ResourceAdmissionDecision::Queued { .. }),
        "held admission must queue, got {held:?}"
    );

    admission.set_admission_held(false).expect("release hold");
    // Clearing the hold lets the queued start promote on the next reconcile.
    admission.release("noop").expect("promote queued");
    assert!(
        admission.queued_jobs().expect("queued jobs").is_empty(),
        "queued start should promote once the hold clears"
    );
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
