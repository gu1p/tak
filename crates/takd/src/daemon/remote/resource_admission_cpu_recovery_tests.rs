#![cfg(test)]

use super::super::resource_envelope::{HostResourceBaseline, calculate_resource_envelope};
use super::super::tak_container_usage::SharedTakContainerUsage;
use super::test_support::request;
use super::{HostUsageSample, ResourceCapacity, SharedResourceAdmission};

#[test]
fn worker_started_under_full_cpu_load_recovers_without_restarting() {
    let envelope = calculate_resource_envelope(HostResourceBaseline {
        total: ResourceCapacity {
            cpu_cores: 12.0,
            memory_mb: 64 * 1024,
        },
        baseline_p95: capacity(12.0),
    });
    let admission = SharedResourceAdmission::new_with_resource_envelope(
        SharedTakContainerUsage::default(),
        envelope,
        1,
        capacity(1.0),
        Some(HostUsageSample {
            non_tak_usage: capacity(12.0),
            available_memory_mb: 48 * 1024,
        }),
    );
    assert!(
        !admission
            .admit_immediately(request("check", 2.0, 1))
            .unwrap()
    );

    admission
        .update_host_usage(capacity(0.4), 48 * 1024)
        .unwrap();
    assert!(
        admission
            .admit_immediately(request("check", 2.0, 1))
            .unwrap()
    );
    let available = admission.resource_snapshot().unwrap().admittable.cpu_cores;
    assert!((available - 7.6).abs() < 0.001, "available={available}");

    admission
        .update_host_usage(capacity(10.0), 48 * 1024)
        .unwrap();
    assert!(
        !admission
            .admit_immediately(request("later", 1.0, 1))
            .unwrap()
    );
    admission.release("check").unwrap();
    assert!(
        !admission
            .admit_immediately(request("too-large", 1.5, 1))
            .unwrap()
    );
    assert!(
        admission
            .admit_immediately(request("fits", 1.0, 1))
            .unwrap()
    );
}

fn capacity(cpu_cores: f64) -> ResourceCapacity {
    ResourceCapacity {
        cpu_cores,
        memory_mb: 1024,
    }
}
