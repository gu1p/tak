#![cfg(test)]

use super::super::tak_container_usage::SharedTakContainerUsage;
use super::test_support::{elastic_request, request};
use super::{ResourceAdmissionDecision, ResourceCapacity, SharedResourceAdmission};

#[path = "resource_admission_tests/tests/snapshots.rs"]
mod snapshots;

#[test]
fn elastic_work_queues_when_startup_capacity_is_not_available() {
    let admission = SharedResourceAdmission::new_for_tests(ResourceCapacity {
        cpu_cores: 4.0,
        memory_mb: 4096,
    });
    admission
        .admit_or_queue(request("running", 4.0, 4096))
        .expect("running admission");

    let decision = admission
        .admit_or_queue(elastic_request("elastic"))
        .expect("elastic admission");

    assert!(matches!(
        decision,
        ResourceAdmissionDecision::Queued { queue_position: 1 }
    ));
}

#[test]
fn aggregate_actual_tak_usage_overrides_lower_reservations() {
    let usage = SharedTakContainerUsage::with_snapshot_for_tests(2.0, 3500 * 1024 * 1024);
    let admission = SharedResourceAdmission::new(
        usage,
        ResourceCapacity {
            cpu_cores: 4.0,
            memory_mb: 4096,
        },
        1,
    );

    let decision = admission
        .admit_or_queue(request("next", 1.0, 1024))
        .expect("admission decision");

    assert!(matches!(decision, ResourceAdmissionDecision::Queued { .. }));
}

#[test]
fn one_reconcile_promotes_only_the_fifo_head() {
    let admission = SharedResourceAdmission::new_for_tests(ResourceCapacity {
        cpu_cores: 4.0,
        memory_mb: 4096,
    });
    admission
        .admit_or_queue(request("running", 4.0, 4096))
        .expect("running admission");
    admission
        .admit_or_queue(request("first", 1.0, 512))
        .expect("first queued admission");
    admission
        .admit_or_queue(request("second", 1.0, 512))
        .expect("second queued admission");

    admission.release("running").expect("resource reconcile");

    let queued = admission.queued_jobs().expect("queued jobs");
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].task_run_id, "second");
}
