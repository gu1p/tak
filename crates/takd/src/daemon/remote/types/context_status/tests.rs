use tak_proto::{NodeStatusResponse, ResourceEnvelopeStatus, ResourcePressureStatus, StorageUsage};

use super::compact_resource_summary;

const MIB: u64 = 1024 * 1024;

#[test]
fn compact_resource_summary_names_every_resource_dimension_unambiguously() {
    let status = NodeStatusResponse {
        storage: Some(StorageUsage {
            available_bytes: 12_345 * MIB,
            ..StorageUsage::default()
        }),
        resource_envelope: Some(ResourceEnvelopeStatus {
            host_cpu_total_cores: 16.0,
            reserve_cpu_cores: 2.0,
            workload_cpu_cores: 14.0,
            tak_usage_cpu_cores: 3.5,
            non_tak_cpu_cores: 1.25,
            reserved_cpu_cores: 5.0,
            admittable_cpu_cores: 9.0,
            host_memory_total_bytes: 32_768 * MIB,
            reserve_memory_bytes: 4_096 * MIB,
            workload_memory_bytes: 28_672 * MIB,
            tak_usage_memory_bytes: 6_144 * MIB,
            non_tak_memory_bytes: 2_048 * MIB,
            reserved_memory_bytes: 8_192 * MIB,
            admittable_memory_bytes: 20_480 * MIB,
            swap_total_bytes: 8_192 * MIB,
            swap_available_bytes: 6_144 * MIB,
        }),
        resource_pressure: Some(ResourcePressureStatus {
            state: "recovering".into(),
            episode_started_at_ms: Some(1_725_000_000_000),
            healthy_samples: 2,
        }),
        ..NodeStatusResponse::default()
    };

    let summary = compact_resource_summary(&status);

    for expected in [
        "host_cpu_total=16.00",
        "reserve_cpu=2.00",
        "workload_cpu=14.00",
        "tak_usage_cpu=3.50",
        "non_tak_cpu=1.25",
        "reserved_cpu=5.00",
        "admittable_cpu=9.00",
        "host_memory_total_mb=32768",
        "reserve_memory_mb=4096",
        "workload_memory_mb=28672",
        "tak_usage_memory_mb=6144",
        "non_tak_memory_mb=2048",
        "reserved_memory_mb=8192",
        "admittable_memory_mb=20480",
        "swap_total_mb=8192",
        "swap_available_mb=6144",
        "pressure=recovering",
        "pressure_episode_started_at_ms=1725000000000",
        "pressure_healthy_samples=2",
        "storage_available_mb=12345",
    ] {
        assert!(
            summary
                .split_ascii_whitespace()
                .any(|part| part == expected),
            "missing {expected}: {summary}"
        );
    }
    assert!(!summary.contains("cpu_available="), "{summary}");
    assert!(!summary.contains("memory_available_mb="), "{summary}");
}
