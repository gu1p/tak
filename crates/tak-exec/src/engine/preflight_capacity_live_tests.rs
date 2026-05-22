#![cfg(test)]

use tak_proto::{CpuUsage, MemoryUsage, NodeStatusResponse};

use super::load_from_status;
use super::test_support::target;

#[test]
fn live_memory_headroom_marks_node_non_fitting() {
    let status = NodeStatusResponse {
        sampled_at_ms: 1,
        cpu: Some(CpuUsage {
            utilization_percent: Some(0.0),
            logical_cores: 8,
            non_tak_used_cores: Some(0.0),
            tak_reserved_cores: Some(0.0),
            tak_admission_available_cores: Some(8.0),
        }),
        memory: Some(MemoryUsage {
            used_bytes: 0,
            total_bytes: 8 * 1024 * 1024 * 1024,
            available_bytes: Some(8 * 1024 * 1024 * 1024),
            non_tak_used_bytes: Some(7 * 1024 * 1024 * 1024),
            tak_reserved_bytes: Some(0),
            tak_admission_available_bytes: Some(256 * 1024 * 1024),
        }),
        ..Default::default()
    };

    let load = load_from_status(&target(), &status);

    assert!(load.status_known);
    assert!(!load.fits_requested_resources);
}

#[test]
fn live_cpu_headroom_marks_node_non_fitting() {
    let status = NodeStatusResponse {
        sampled_at_ms: 1,
        cpu: Some(CpuUsage {
            utilization_percent: Some(95.0),
            logical_cores: 8,
            non_tak_used_cores: Some(7.6),
            tak_reserved_cores: Some(0.0),
            tak_admission_available_cores: Some(0.4),
        }),
        memory: Some(MemoryUsage {
            used_bytes: 0,
            total_bytes: 8 * 1024 * 1024 * 1024,
            available_bytes: Some(8 * 1024 * 1024 * 1024),
            non_tak_used_bytes: Some(0),
            tak_reserved_bytes: Some(0),
            tak_admission_available_bytes: Some(8 * 1024 * 1024 * 1024),
        }),
        ..Default::default()
    };

    let load = load_from_status(&target(), &status);

    assert!(load.status_known);
    assert!(!load.fits_requested_resources);
}
