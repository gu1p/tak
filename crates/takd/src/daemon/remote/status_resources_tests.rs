#![cfg(test)]

use super::{
    cpu_admission_available, host_cpu_cores_used, memory_admission_available, non_tak_cpu_cores,
    non_tak_memory_bytes,
};

#[test]
fn cpu_available_buffers_non_tak_usage_only() {
    let host_used = host_cpu_cores_used(75.0, 8);
    let non_tak_used = non_tak_cpu_cores(host_used, 2.0);

    let available = cpu_admission_available(8, non_tak_used, 1.0);

    assert!((available - 2.6).abs() < 0.001);
}

#[test]
fn memory_available_buffers_non_tak_usage_only() {
    let mib = 1024 * 1024;
    let non_tak_used = non_tak_memory_bytes(6 * mib, 2 * mib);

    let available = memory_admission_available(10 * mib, non_tak_used, 3 * mib);
    let expected = 10 * mib - ((4 * mib) as f64 * 1.10).ceil() as u64 - 3 * mib;

    assert_eq!(available, expected);
}
