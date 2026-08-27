#![cfg(test)]

use super::super::resource_envelope::ResourceEnvelope;
use super::super::tak_container_usage::SharedTakContainerUsage;
use super::test_support::request;
use super::{
    HostUsageSample, ResourceAdmissionDecision, ResourceCapacity, SharedResourceAdmission,
};

#[test]
fn work_queues_until_the_first_host_usage_sample() {
    let admission = admission(None);

    let decision = admission
        .admit_or_queue(request("waiting", 1.0, 1))
        .expect("admission decision");

    assert!(matches!(decision, ResourceAdmissionDecision::Queued { .. }));
}

#[test]
fn first_host_usage_sample_promotes_queued_work() {
    let admission = admission(None);
    admission
        .admit_or_queue(request("waiting", 1.0, 1))
        .expect("queue work before the first sample");

    admission
        .update_host_usage(capacity(0.0, 0), 16)
        .expect("record first host usage sample");

    assert!(
        admission.queued_jobs().expect("queued work").is_empty(),
        "the first usable host sample should wake queued work"
    );
    let snapshot = admission.resource_snapshot().expect("resource snapshot");
    assert_eq!(snapshot.reserved, capacity(1.0, 1));
}

#[test]
fn host_use_inside_the_reserve_does_not_shrink_the_workload_envelope() {
    let admission = admission(Some(HostUsageSample {
        non_tak_usage: capacity(2.0, 2),
        available_memory_mb: 16,
    }));

    let decision = admission
        .admit_or_queue(request("fits", 12.0, 12))
        .expect("admission decision");

    assert!(matches!(decision, ResourceAdmissionDecision::Admitted));
}

#[test]
fn status_snapshot_uses_the_host_sample_that_controls_admission() {
    let host_usage = HostUsageSample {
        non_tak_usage: capacity(3.0, 4),
        available_memory_mb: 2,
    };
    let admission = admission(Some(host_usage));

    let snapshot = admission.resource_snapshot().expect("resource snapshot");

    assert_eq!(snapshot.host_usage, Some(host_usage));
    assert_eq!(snapshot.admittable.memory_mb, 2);
}

fn admission(host_usage: Option<HostUsageSample>) -> SharedResourceAdmission {
    SharedResourceAdmission::new_with_resource_envelope(
        SharedTakContainerUsage::default(),
        ResourceEnvelope {
            total: capacity(16.0, 16),
            margin: capacity(1.0, 1),
            host_reserve: capacity(4.0, 4),
            workload: capacity(12.0, 12),
        },
        1,
        capacity(1.0, 1),
        host_usage,
    )
}

fn capacity(cpu_cores: f64, memory_mb: u64) -> ResourceCapacity {
    ResourceCapacity {
        cpu_cores,
        memory_mb,
    }
}
