const EXTERNAL_USAGE_BUFFER_RATIO: f64 = 1.10;

#[path = "status_resources_tests.rs"]
mod tests;

pub(super) fn host_cpu_cores_used(utilization_percent: f64, logical_cores: u32) -> f64 {
    utilization_percent / 100.0 * f64::from(logical_cores)
}

pub(super) fn non_tak_cpu_cores(host_cpu_cores_used: f64, tak_cpu_cores: f64) -> f64 {
    (host_cpu_cores_used - tak_cpu_cores).max(0.0)
}

pub(super) fn cpu_admission_available(
    logical_cores: u32,
    non_tak_used_cores: f64,
    tak_reserved_cores: f64,
) -> f64 {
    (f64::from(logical_cores) - buffered_external_usage(non_tak_used_cores) - tak_reserved_cores)
        .max(0.0)
}

pub(super) fn non_tak_memory_bytes(host_used_bytes: u64, tak_memory_bytes: u64) -> u64 {
    host_used_bytes.saturating_sub(tak_memory_bytes)
}

pub(super) fn memory_admission_available(
    total_bytes: u64,
    non_tak_used_bytes: u64,
    tak_reserved_bytes: u64,
) -> u64 {
    let buffered_external = buffered_external_usage(non_tak_used_bytes as f64).ceil() as u64;
    let protected = buffered_external.saturating_add(tak_reserved_bytes);
    total_bytes.saturating_sub(protected)
}

fn buffered_external_usage(value: f64) -> f64 {
    value * EXTERNAL_USAGE_BUFFER_RATIO
}
