use sysinfo::System;

use crate::daemon::remote::tak_container_usage::SharedTakContainerUsage;

use super::EXTERNAL_USAGE_BUFFER_RATIO;

const BYTES_PER_MB: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy)]
pub(super) struct ResourceCapacity {
    pub(super) cpu_cores: f64,
    pub(super) memory_mb: u64,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ResourceUsageSnapshot {
    pub(super) tak_cpu_cores: f64,
    pub(super) tak_memory_mb: u64,
    pub(super) host_cpu_cores_used: f64,
    pub(super) host_memory_mb_used: u64,
}

pub(super) enum ResourceUsageSource {
    Detected {
        system: Box<System>,
        cpu_sample_ready: bool,
        tak_container_usage: SharedTakContainerUsage,
    },
    #[cfg_attr(not(test), allow(dead_code))]
    Fixed(ResourceUsageSnapshot),
}

impl ResourceUsageSource {
    pub(super) fn snapshot(&mut self, capacity: ResourceCapacity) -> ResourceUsageSnapshot {
        match self {
            ResourceUsageSource::Detected {
                system,
                cpu_sample_ready,
                tak_container_usage,
            } => detected_resource_usage(
                system.as_mut(),
                cpu_sample_ready,
                tak_container_usage,
                capacity,
            ),
            ResourceUsageSource::Fixed(snapshot) => *snapshot,
        }
    }
}

fn detected_resource_usage(
    system: &mut System,
    cpu_sample_ready: &mut bool,
    tak_container_usage: &SharedTakContainerUsage,
    capacity: ResourceCapacity,
) -> ResourceUsageSnapshot {
    system.refresh_memory();
    system.refresh_cpu_usage();
    let host_cpu_cores_used = if *cpu_sample_ready {
        f64::from(system.global_cpu_usage()) / 100.0 * capacity.cpu_cores
    } else {
        0.0
    };
    *cpu_sample_ready = true;
    let tak_usage = tak_container_usage.latest();
    ResourceUsageSnapshot {
        tak_cpu_cores: tak_usage.cpu_cores,
        tak_memory_mb: tak_usage.memory_bytes / BYTES_PER_MB,
        host_cpu_cores_used,
        host_memory_mb_used: system
            .total_memory()
            .saturating_sub(system.available_memory())
            / BYTES_PER_MB,
    }
}

pub(super) fn protected_cpu_usage(tak_cpu_cores: f64, host_cpu_cores_used: f64) -> f64 {
    (host_cpu_cores_used - tak_cpu_cores).max(0.0) * EXTERNAL_USAGE_BUFFER_RATIO
}

pub(super) fn protected_memory_usage(tak_memory_mb: u64, host_memory_mb_used: u64) -> u64 {
    let non_tak_memory_mb = host_memory_mb_used.saturating_sub(tak_memory_mb);
    ((non_tak_memory_mb as f64) * EXTERNAL_USAGE_BUFFER_RATIO).ceil() as u64
}
