use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tak_core::model::ContainerResourceLimitsSpec;

use super::memory_pressure_controller::resume_headroom_bytes;
use super::resource_admission::ResourceCapacity;
use super::runtime::RemoteRuntimeConfig;

#[derive(Debug, Clone)]
pub(super) struct RemoteResourcePolicy {
    capacity: ResourceCapacity,
    default_cpu_cores: f64,
    default_memory_mb: u64,
}

impl RemoteResourcePolicy {
    pub(super) fn detected(config: &RemoteRuntimeConfig) -> Self {
        let mut system = System::new_with_specifics(
            RefreshKind::nothing()
                .with_cpu(CpuRefreshKind::everything())
                .with_memory(MemoryRefreshKind::everything()),
        );
        system.refresh_memory();
        system.refresh_cpu_all();
        let total_bytes = system.total_memory();
        let safe_memory_bytes = total_bytes.saturating_sub(resume_headroom_bytes(
            &config.memory_pressure(),
            total_bytes,
        ));
        Self::new(
            ResourceCapacity {
                cpu_cores: system.cpus().len().max(1) as f64,
                memory_mb: (safe_memory_bytes / 1024 / 1024).max(1),
            },
            config.default_container_cpu_cores(),
            config.default_container_memory_mb(),
        )
    }

    pub(super) fn new(
        capacity: ResourceCapacity,
        default_cpu_cores: f64,
        default_memory_mb: u64,
    ) -> Self {
        Self {
            capacity,
            default_cpu_cores,
            default_memory_mb,
        }
    }

    pub(super) fn capacity(&self) -> ResourceCapacity {
        self.capacity
    }

    pub(super) fn resolve(
        &self,
        authored: Option<ContainerResourceLimitsSpec>,
    ) -> ContainerResourceLimitsSpec {
        authored.unwrap_or(ContainerResourceLimitsSpec {
            cpu_cores: Some(self.default_cpu_cores.min(self.capacity.cpu_cores)),
            memory_mb: Some(self.default_memory_mb.min(self.capacity.memory_mb)),
        })
    }
}
