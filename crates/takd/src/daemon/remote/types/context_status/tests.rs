use tak_proto::{CpuUsage, MemoryUsage, NodeStatusResponse};

use super::compact_resource_summary;

#[test]
fn compact_resource_summary_includes_available_and_total_capacity() {
    let status = NodeStatusResponse {
        cpu: Some(CpuUsage {
            logical_cores: 8,
            tak_admission_available_cores: Some(2.0),
            ..CpuUsage::default()
        }),
        memory: Some(MemoryUsage {
            total_bytes: 16 * 1024 * 1024 * 1024,
            tak_admission_available_bytes: Some(4 * 1024 * 1024 * 1024),
            ..MemoryUsage::default()
        }),
        ..NodeStatusResponse::default()
    };

    let summary = compact_resource_summary(&status);

    assert!(summary.contains("cpu_available=2.00"));
    assert!(summary.contains("cpu_total=8.00"));
    assert!(summary.contains("memory_available_mb=4096"));
    assert!(summary.contains("memory_total_mb=16384"));
}
