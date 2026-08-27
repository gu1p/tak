use std::time::Duration;

use tak_core::model::ContainerResourceLimitsSpec;

use super::resource_admission::ResourceCapacity;
use super::resource_baseline::detect_host_resource_baseline;
use super::resource_envelope::{
    ElasticAdmissionClaim, ElasticClaimPolicy, calculate_resource_envelope,
};
use super::runtime::RemoteRuntimeConfig;

#[derive(Debug, Clone)]
pub(super) struct RemoteResourcePolicy {
    capacity: ResourceCapacity,
    envelope: super::resource_envelope::ResourceEnvelope,
    default_cpu_cores: f64,
    default_memory_mb: u64,
}

impl RemoteResourcePolicy {
    pub(super) fn detected(config: &RemoteRuntimeConfig) -> Self {
        let envelope = calculate_resource_envelope(detect_host_resource_baseline(config));
        Self::with_envelope(
            envelope,
            config.default_container_cpu_cores(),
            config.default_container_memory_mb(),
        )
    }

    pub(super) fn with_envelope(
        envelope: super::resource_envelope::ResourceEnvelope,
        default_cpu_cores: f64,
        default_memory_mb: u64,
    ) -> Self {
        Self {
            capacity: envelope.workload,
            envelope,
            default_cpu_cores,
            default_memory_mb,
        }
    }

    pub(super) fn envelope(&self) -> super::resource_envelope::ResourceEnvelope {
        self.envelope
    }

    pub(super) fn resolve(
        &self,
        authored: Option<ContainerResourceLimitsSpec>,
    ) -> ContainerResourceLimitsSpec {
        authored.unwrap_or(ContainerResourceLimitsSpec {
            cpu_cores: None,
            memory_mb: None,
        })
    }

    pub(super) fn startup_claim(&self, authored: &ContainerResourceLimitsSpec) -> ResourceCapacity {
        if authored.cpu_cores.is_some() || authored.memory_mb.is_some() {
            return ResourceCapacity {
                cpu_cores: authored.cpu_cores.unwrap_or(self.default_cpu_cores),
                memory_mb: authored.memory_mb.unwrap_or(self.default_memory_mb),
            };
        }
        let policy = ElasticClaimPolicy::new(ResourceCapacity {
            cpu_cores: self.default_cpu_cores,
            memory_mb: self.default_memory_mb,
        });
        match policy.claim_at(Duration::ZERO, Some(self.capacity), self.capacity) {
            ElasticAdmissionClaim::Startup(claim) | ElasticAdmissionClaim::Measured(claim) => claim,
        }
    }
}
