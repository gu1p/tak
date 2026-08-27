#![cfg(test)]

use std::time::Duration;

use super::super::tak_container_usage::SharedTakContainerUsage;
use super::test_support::{elastic_request, request};
use super::{ResourceAdmissionDecision, ResourceCapacity, SharedResourceAdmission};

#[test]
fn zero_workload_envelope_queues_elastic_work() {
    let admission = SharedResourceAdmission::new_for_tests(capacity(0.0, 0));

    let decision = admission
        .admit_or_queue(elastic_request("elastic"))
        .expect("admission decision");

    assert!(matches!(decision, ResourceAdmissionDecision::Queued { .. }));
}

#[test]
fn pending_startup_is_added_after_actual_usage_overrides_reservations() {
    let usage = SharedTakContainerUsage::with_snapshot_for_tests(9.0, 9 * 1024 * 1024);
    let admission = SharedResourceAdmission::new_with_elastic_startup(
        usage,
        capacity(12.0, 12),
        1,
        capacity(1.0, 1),
    );
    admission
        .admit_or_queue(request("reserved", 2.0, 2))
        .expect("reserved admission");
    admission
        .admit_or_queue(elastic_request("starting"))
        .expect("elastic admission");

    let snapshot = admission.resource_snapshot().expect("resource snapshot");

    assert_eq!(snapshot.admittable, capacity(2.0, 2));
}

#[path = "resource_admission_safety_tests/tests/startup_claim.rs"]
mod startup_claim;

#[test]
fn authored_commitment_and_elastic_usage_are_both_claimed_at_strict_capacity() {
    const MIB: u64 = 1024 * 1024;
    let usage = SharedTakContainerUsage::default();
    let admission = SharedResourceAdmission::new_with_elastic_startup(
        usage.clone(),
        capacity(8.0, 8),
        1,
        capacity(1.0, 1),
    );
    admission
        .admit_or_queue(request("authored", 4.0, 4))
        .expect("authored admission");
    admission
        .admit_or_queue(elastic_request("elastic"))
        .expect("elastic admission");
    usage.set_task_snapshots_for_tests(&[("authored", 2.0, 2 * MIB), ("elastic", 3.0, 3 * MIB)]);
    admission.age_admission_for_tests("elastic", Duration::from_secs(6));

    let snapshot = admission.resource_snapshot().expect("resource snapshot");

    assert_eq!(snapshot.reserved, capacity(4.0, 4));
    assert_eq!(snapshot.pending_startup, capacity(0.0, 0));
    assert_eq!(snapshot.actual, capacity(5.0, 5));
    assert_eq!(snapshot.admittable, capacity(1.0, 1));
}

fn capacity(cpu_cores: f64, memory_mb: u64) -> ResourceCapacity {
    ResourceCapacity {
        cpu_cores,
        memory_mb,
    }
}
