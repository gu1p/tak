use std::thread;
use std::time::Duration;

use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

use super::resource_admission::ResourceCapacity;
use super::resource_envelope::HostResourceBaseline;
use super::runtime::RemoteRuntimeConfig;
use super::status_resources::effective_available_memory;

const MAX_BASELINE_SAMPLES: u32 = 20;
const BYTES_PER_MIB: u64 = 1024 * 1024;

pub(super) fn detect_host_resource_baseline(config: &RemoteRuntimeConfig) -> HostResourceBaseline {
    let mut system = System::new_with_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );
    system.refresh_memory();
    system.refresh_cpu_all();
    let total = total_capacity(&system);
    let window = config.host_baseline_sample_duration();
    let samples = sample_host_usage(&mut system, total.cpu_cores, window);
    let baseline_p95 = baseline_p95(&samples);
    tracing::info!(
        sample_ms = window.as_millis(),
        cpu_cores = baseline_p95.cpu_cores,
        memory_mb = baseline_p95.memory_mb,
        "measured host resource baseline"
    );
    HostResourceBaseline {
        total,
        baseline_p95,
    }
}

pub(super) fn baseline_p95(samples: &[ResourceCapacity]) -> ResourceCapacity {
    if samples.is_empty() {
        return ResourceCapacity {
            cpu_cores: 0.0,
            memory_mb: 0,
        };
    }
    let rank = samples.len().saturating_mul(95).div_ceil(100) - 1;
    let mut cpu = samples
        .iter()
        .map(|sample| sample.cpu_cores)
        .collect::<Vec<_>>();
    let mut memory = samples
        .iter()
        .map(|sample| sample.memory_mb)
        .collect::<Vec<_>>();
    cpu.sort_by(f64::total_cmp);
    memory.sort_unstable();
    ResourceCapacity {
        cpu_cores: cpu[rank],
        memory_mb: memory[rank],
    }
}

fn total_capacity(system: &System) -> ResourceCapacity {
    ResourceCapacity {
        cpu_cores: system.cpus().len().max(1) as f64,
        memory_mb: system.total_memory().div_ceil(BYTES_PER_MIB).max(1),
    }
}

fn sample_host_usage(
    system: &mut System,
    logical_cores: f64,
    window: Duration,
) -> Vec<ResourceCapacity> {
    if window.is_zero() {
        return Vec::new();
    }
    let count = sample_count(window);
    let interval = window / count;
    (0..count)
        .map(|_| {
            thread::sleep(interval);
            system.refresh_cpu_usage();
            system.refresh_memory();
            let total_memory = system.total_memory();
            let available_memory = effective_available_memory(
                total_memory,
                system.used_memory(),
                system.available_memory(),
            );
            ResourceCapacity {
                cpu_cores: f64::from(system.global_cpu_usage()) * logical_cores / 100.0,
                memory_mb: total_memory
                    .saturating_sub(available_memory)
                    .div_ceil(BYTES_PER_MIB),
            }
        })
        .collect()
}

fn sample_count(window: Duration) -> u32 {
    let minimum = sysinfo::MINIMUM_CPU_UPDATE_INTERVAL.max(Duration::from_millis(1));
    let available = window.as_nanos() / minimum.as_nanos();
    available.clamp(1, u128::from(MAX_BASELINE_SAMPLES)) as u32
}
