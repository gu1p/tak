#![cfg(test)]

use super::stats::{cpu_cores_from_deltas, required_memory_usage};
use super::{SharedTakContainerUsage, TakContainerUsageSnapshot, TakTaskUsageSnapshot};

impl SharedTakContainerUsage {
    pub(crate) fn with_snapshot_for_tests(cpu_cores: f64, memory_bytes: u64) -> Self {
        let usage = Self::default();
        usage.update(TakContainerUsageSnapshot {
            cpu_cores,
            memory_bytes,
            sampled_at: None,
            task_usage: std::collections::HashMap::new(),
            attribution_complete: false,
        });
        usage
    }

    pub(crate) fn set_task_snapshots_for_tests(&self, snapshots: &[(&str, f64, u64)]) {
        let cpu_cores = snapshots.iter().map(|(_, cpu, _)| cpu).sum();
        let memory_bytes = snapshots
            .iter()
            .fold(0_u64, |total, (_, _, memory)| total.saturating_add(*memory));
        self.update(TakContainerUsageSnapshot {
            cpu_cores,
            memory_bytes,
            sampled_at: None,
            task_usage: snapshots
                .iter()
                .map(|(key, cpu_cores, memory_bytes)| {
                    (
                        (*key).to_string(),
                        TakTaskUsageSnapshot {
                            cpu_cores: *cpu_cores,
                            memory_bytes: *memory_bytes,
                        },
                    )
                })
                .collect(),
            attribution_complete: true,
        });
    }
}

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
