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
    _non_tak_used_cores: f64,
    tak_reserved_cores: f64,
) -> f64 {
    (f64::from(logical_cores) - tak_reserved_cores).max(0.0)
}

pub(super) fn non_tak_memory_bytes(host_used_bytes: u64, tak_memory_bytes: u64) -> u64 {
    host_used_bytes.saturating_sub(tak_memory_bytes)
}

pub(super) fn memory_admission_available(
    total_bytes: u64,
    _non_tak_used_bytes: u64,
    tak_reserved_bytes: u64,
) -> u64 {
    total_bytes.saturating_sub(tak_reserved_bytes)
}
