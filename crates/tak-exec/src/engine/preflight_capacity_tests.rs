#![cfg(test)]

use tak_proto::{CpuUsage, MemoryUsage, NodeStatusResponse};

use super::load_from_status;
use super::test_support::{queued_job, target};

#[test]
fn queued_jobs_do_not_consume_capacity_for_load_fits() {
    let status = NodeStatusResponse {
        sampled_at_ms: 1,
        cpu: Some(CpuUsage {
            utilization_percent: Some(0.0),
            logical_cores: 2,
            ..Default::default()
        }),
        memory: Some(MemoryUsage {
            used_bytes: 0,
            total_bytes: 1024 * 1024 * 1024,
            ..Default::default()
        }),
        storage: None,
        queued_jobs: vec![queued_job(), queued_job()],
        ..Default::default()
    };

    let load = load_from_status(&target(), &status);

    assert!(load.status_known);
    assert!(load.fits_requested_resources);
    assert_eq!(load.job_count, 2);
    assert_eq!(load.cpu_ratio, 0.0);
    assert_eq!(load.memory_ratio, 0.0);
}

#[test]
fn zero_or_missing_capacity_is_unknown_load() {
    let missing = NodeStatusResponse::default();
    let zero = NodeStatusResponse {
        cpu: Some(CpuUsage {
            utilization_percent: Some(0.0),
            logical_cores: 0,
            ..Default::default()
        }),
        memory: Some(MemoryUsage {
            used_bytes: 0,
            total_bytes: 0,
            ..Default::default()
        }),
        ..missing.clone()
    };

    assert!(!load_from_status(&target(), &missing).status_known);
    assert!(!load_from_status(&target(), &zero).status_known);
}
