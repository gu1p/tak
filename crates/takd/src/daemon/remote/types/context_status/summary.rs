use tak_proto::NodeStatusResponse;

pub(super) fn compact_resource_summary(status: &NodeStatusResponse) -> String {
    let envelope = status
        .resource_envelope
        .as_ref()
        .map(resource_envelope_summary)
        .unwrap_or_else(|| "resource_envelope=unknown".to_string());
    let pressure = status
        .resource_pressure
        .as_ref()
        .map(resource_pressure_summary)
        .unwrap_or_else(|| "pressure=unknown".to_string());
    let storage = status
        .storage
        .as_ref()
        .map(storage_summary)
        .unwrap_or_else(|| "storage=unknown".to_string());
    format!("{envelope} {pressure} {storage}")
}

fn resource_envelope_summary(envelope: &tak_proto::ResourceEnvelopeStatus) -> String {
    const MIB: u64 = 1024 * 1024;
    format!(
        concat!(
            "host_cpu_total={:.2} reserve_cpu={:.2} workload_cpu={:.2} ",
            "tak_usage_cpu={:.2} non_tak_cpu={:.2} reserved_cpu={:.2} ",
            "admittable_cpu={:.2} host_memory_total_mb={} reserve_memory_mb={} ",
            "workload_memory_mb={} tak_usage_memory_mb={} non_tak_memory_mb={} ",
            "reserved_memory_mb={} admittable_memory_mb={} swap_total_mb={} ",
            "swap_available_mb={}"
        ),
        envelope.host_cpu_total_cores,
        envelope.reserve_cpu_cores,
        envelope.workload_cpu_cores,
        envelope.tak_usage_cpu_cores,
        envelope.non_tak_cpu_cores,
        envelope.reserved_cpu_cores,
        envelope.admittable_cpu_cores,
        envelope.host_memory_total_bytes / MIB,
        envelope.reserve_memory_bytes / MIB,
        envelope.workload_memory_bytes / MIB,
        envelope.tak_usage_memory_bytes / MIB,
        envelope.non_tak_memory_bytes / MIB,
        envelope.reserved_memory_bytes / MIB,
        envelope.admittable_memory_bytes / MIB,
        envelope.swap_total_bytes / MIB,
        envelope.swap_available_bytes / MIB,
    )
}

fn resource_pressure_summary(pressure: &tak_proto::ResourcePressureStatus) -> String {
    let episode = pressure
        .episode_started_at_ms
        .map(|value| value.to_string())
        .unwrap_or_else(|| "none".to_string());
    format!(
        "pressure={} pressure_episode_started_at_ms={} pressure_healthy_samples={}",
        pressure.state, episode, pressure.healthy_samples
    )
}

fn storage_summary(storage: &tak_proto::StorageUsage) -> String {
    format!(
        "storage_available_mb={}",
        storage.available_bytes / 1024 / 1024
    )
}
