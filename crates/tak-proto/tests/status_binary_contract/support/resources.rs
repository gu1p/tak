use tak_proto::{ResourceEnvelopeStatus, ResourcePressureStatus};

pub(super) fn resource_envelope() -> ResourceEnvelopeStatus {
    ResourceEnvelopeStatus {
        host_cpu_total_cores: 8.0,
        reserve_cpu_cores: 1.6,
        workload_cpu_cores: 6.4,
        tak_usage_cpu_cores: 2.0,
        non_tak_cpu_cores: 1.0,
        reserved_cpu_cores: 2.0,
        admittable_cpu_cores: 3.4,
        host_memory_total_bytes: 8_192,
        reserve_memory_bytes: 2_048,
        workload_memory_bytes: 6_144,
        tak_usage_memory_bytes: 1_024,
        non_tak_memory_bytes: 1_024,
        reserved_memory_bytes: 2_048,
        admittable_memory_bytes: 3_072,
        swap_total_bytes: 4_096,
        swap_available_bytes: 3_072,
    }
}

pub(super) fn resource_pressure() -> ResourcePressureStatus {
    ResourcePressureStatus {
        state: "healthy".to_string(),
        episode_started_at_ms: None,
        healthy_samples: 4,
    }
}
