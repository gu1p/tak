#![cfg(test)]

use super::test_support::request;
use super::usage::ResourceUsageSnapshot;
use super::{ResourceAdmissionDecision, ResourceCapacity, SharedResourceAdmission};

#[test]
fn external_memory_usage_is_buffered_before_admitting_new_work() {
    let admission = SharedResourceAdmission::new_for_tests(
        ResourceCapacity {
            cpu_cores: 8.0,
            memory_mb: 1024,
        },
        ResourceUsageSnapshot {
            tak_cpu_cores: 0.0,
            tak_memory_mb: 0,
            host_cpu_cores_used: 0.0,
            host_memory_mb_used: 600,
        },
    );

    let decision = admission
        .admit_or_queue(request("memory-heavy", 1.0, 400))
        .expect("admission decision");

    assert!(matches!(
        decision,
        ResourceAdmissionDecision::Queued { queue_position: 1 }
    ));
}

#[test]
fn external_cpu_usage_is_buffered_before_admitting_new_work() {
    let admission = SharedResourceAdmission::new_for_tests(
        ResourceCapacity {
            cpu_cores: 8.0,
            memory_mb: 4096,
        },
        ResourceUsageSnapshot {
            tak_cpu_cores: 0.0,
            tak_memory_mb: 0,
            host_cpu_cores_used: 6.8,
            host_memory_mb_used: 0,
        },
    );

    let decision = admission
        .admit_or_queue(request("cpu-heavy", 1.0, 512))
        .expect("admission decision");

    assert!(matches!(
        decision,
        ResourceAdmissionDecision::Queued { queue_position: 1 }
    ));
}

#[test]
fn disabled_external_usage_protection_admits_simulated_work_under_host_load() {
    let admission = SharedResourceAdmission::new_for_tests_with_host_protection(
        ResourceCapacity {
            cpu_cores: 8.0,
            memory_mb: 4096,
        },
        ResourceUsageSnapshot {
            tak_cpu_cores: 0.0,
            tak_memory_mb: 0,
            host_cpu_cores_used: 7.9,
            host_memory_mb_used: 4000,
        },
        false,
    );

    let decision = admission
        .admit_or_queue(request("streaming-contract", 1.0, 512))
        .expect("admission decision");

    assert!(matches!(decision, ResourceAdmissionDecision::Admitted));
}
