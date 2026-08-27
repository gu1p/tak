use std::time::Duration;

use super::resource_admission::ResourceCapacity;
use super::resource_envelope::{
    ElasticAdmissionClaim, ElasticClaimPolicy, HostResourceBaseline, calculate_resource_envelope,
};

#[test]
fn percentage_reserve_dominates_on_an_idle_large_worker() {
    let envelope = calculate_resource_envelope(HostResourceBaseline {
        total: capacity(20.0, 40 * 1024),
        baseline_p95: capacity(0.25, 512),
    });

    assert_capacity(envelope.margin, 1.0, 2 * 1024);
    assert_capacity(envelope.host_reserve, 4.0, 8 * 1024);
    assert_capacity(envelope.workload, 16.0, 32 * 1024);
}

#[test]
fn baseline_plus_margin_dominates_on_a_busy_worker() {
    let envelope = calculate_resource_envelope(HostResourceBaseline {
        total: capacity(20.0, 40 * 1024),
        baseline_p95: capacity(8.0, 15 * 1024),
    });

    assert_capacity(envelope.margin, 1.0, 2 * 1024);
    assert_capacity(envelope.host_reserve, 9.0, 17 * 1024);
    assert_capacity(envelope.workload, 11.0, 23 * 1024);
}

#[test]
fn absolute_floors_dominate_on_a_small_worker() {
    let envelope = calculate_resource_envelope(HostResourceBaseline {
        total: capacity(2.0, 4 * 1024),
        baseline_p95: capacity(0.0, 0),
    });

    assert_capacity(envelope.margin, 0.5, 1024);
    assert_capacity(envelope.host_reserve, 1.0, 2 * 1024);
    assert_capacity(envelope.workload, 1.0, 2 * 1024);
}

#[test]
fn elastic_claim_is_temporary_measured_and_clamped() {
    let policy = ElasticClaimPolicy::default();
    let full_startup = policy.claim_at(
        Duration::ZERO,
        Some(capacity(0.25, 256)),
        capacity(8.0, 16 * 1024),
    );
    let clamped_startup = policy.claim_at(
        Duration::from_secs(4),
        Some(capacity(0.25, 256)),
        capacity(2.0, 6 * 1024),
    );
    let unmeasured = policy.claim_at(Duration::from_secs(6), None, capacity(2.0, 6 * 1024));
    let measured = policy.claim_at(
        Duration::from_secs(5),
        Some(capacity(0.75, 768)),
        capacity(2.0, 6 * 1024),
    );

    assert_claim(full_startup, true, 4.0, 8 * 1024);
    assert_claim(clamped_startup, true, 2.0, 6 * 1024);
    assert_claim(unmeasured, true, 2.0, 6 * 1024);
    assert_claim(measured, false, 0.75, 768);
}

fn capacity(cpu_cores: f64, memory_mb: u64) -> ResourceCapacity {
    ResourceCapacity {
        cpu_cores,
        memory_mb,
    }
}

fn assert_capacity(actual: ResourceCapacity, cpu_cores: f64, memory_mb: u64) {
    assert!((actual.cpu_cores - cpu_cores).abs() < f64::EPSILON);
    assert_eq!(actual.memory_mb, memory_mb);
}

fn assert_claim(actual: ElasticAdmissionClaim, startup: bool, cpu_cores: f64, memory_mb: u64) {
    let capacity = match actual {
        ElasticAdmissionClaim::Startup(capacity) if startup => capacity,
        ElasticAdmissionClaim::Measured(capacity) if !startup => capacity,
        other => panic!("unexpected elastic claim phase: {other:?}"),
    };
    assert_capacity(capacity, cpu_cores, memory_mb);
}
