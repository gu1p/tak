#![cfg(test)]

use super::stats::{cpu_cores_from_deltas, required_memory_usage};

#[test]
fn cpu_cores_are_derived_from_docker_stat_deltas() {
    let cores = cpu_cores_from_deltas(500, 100, Some(2_000), Some(1_000), Some(4), None);

    assert!((cores - 1.6).abs() < 0.001);
}

#[test]
fn cpu_cores_are_zero_without_a_usable_delta() {
    let cores = cpu_cores_from_deltas(500, 500, Some(2_000), Some(1_000), Some(4), None);

    assert_eq!(cores, 0.0);
}

#[test]
fn missing_docker_memory_usage_rejects_the_sample() {
    assert!(required_memory_usage(None).is_err());
}
