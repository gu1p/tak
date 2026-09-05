use std::time::Duration;

use super::resource_admission::ResourceCapacity;

const CPU_RESERVE_FLOOR: f64 = 1.0;
const CPU_MARGIN_FLOOR: f64 = 0.5;
const MEMORY_RESERVE_FLOOR_MB: u64 = 2 * 1024;
const MEMORY_MARGIN_FLOOR_MB: u64 = 1024;
pub(super) const ELASTIC_STARTUP_DURATION: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy)]
pub(super) struct HostResourceBaseline {
    pub(super) total: ResourceCapacity,
    pub(super) baseline_p95: ResourceCapacity,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ResourceEnvelope {
    pub(super) total: ResourceCapacity,
    pub(super) margin: ResourceCapacity,
    pub(super) host_reserve: ResourceCapacity,
    pub(super) workload: ResourceCapacity,
}

pub(super) fn calculate_resource_envelope(baseline: HostResourceBaseline) -> ResourceEnvelope {
    let margin = ResourceCapacity {
        cpu_cores: (baseline.total.cpu_cores * 0.05).max(CPU_MARGIN_FLOOR),
        memory_mb: percent_ceil(baseline.total.memory_mb, 5).max(MEMORY_MARGIN_FLOOR_MB),
    };
    let host_reserve = ResourceCapacity {
        cpu_cores: (baseline.total.cpu_cores * 0.20)
            .max(CPU_RESERVE_FLOOR)
            .min(baseline.total.cpu_cores),
        memory_mb: percent_ceil(baseline.total.memory_mb, 20)
            .max(MEMORY_RESERVE_FLOOR_MB)
            .max(
                baseline
                    .baseline_p95
                    .memory_mb
                    .saturating_add(margin.memory_mb),
            )
            .min(baseline.total.memory_mb),
    };
    let workload = ResourceCapacity {
        cpu_cores: (baseline.total.cpu_cores - host_reserve.cpu_cores).max(0.0),
        memory_mb: baseline
            .total
            .memory_mb
            .saturating_sub(host_reserve.memory_mb),
    };
    ResourceEnvelope {
        total: baseline.total,
        margin,
        host_reserve,
        workload,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ElasticAdmissionClaim {
    Startup(ResourceCapacity),
    Measured(ResourceCapacity),
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ElasticClaimPolicy {
    startup: ResourceCapacity,
    startup_duration: Duration,
}

impl ElasticClaimPolicy {
    pub(super) fn new(startup: ResourceCapacity) -> Self {
        Self {
            startup,
            startup_duration: ELASTIC_STARTUP_DURATION,
        }
    }

    pub(super) fn claim_at(
        &self,
        elapsed: Duration,
        measured: Option<ResourceCapacity>,
        workload: ResourceCapacity,
    ) -> ElasticAdmissionClaim {
        if elapsed < self.startup_duration || measured.is_none() {
            return ElasticAdmissionClaim::Startup(clamp(self.startup, workload));
        }
        ElasticAdmissionClaim::Measured(measured.unwrap_or(workload))
    }
}

impl Default for ElasticClaimPolicy {
    fn default() -> Self {
        Self::new(ResourceCapacity {
            cpu_cores: 4.0,
            memory_mb: 8 * 1024,
        })
    }
}

fn clamp(value: ResourceCapacity, limit: ResourceCapacity) -> ResourceCapacity {
    ResourceCapacity {
        cpu_cores: value.cpu_cores.min(limit.cpu_cores),
        memory_mb: value.memory_mb.min(limit.memory_mb),
    }
}

fn percent_ceil(value: u64, percent: u64) -> u64 {
    value.saturating_mul(percent).div_ceil(100)
}
