use super::*;
use crate::daemon::remote::resource_admission::HostUsageSample;
use crate::daemon::remote::resource_envelope::ResourceEnvelope;

impl SharedResourceAdmission {
    pub(in crate::daemon::remote) fn new_with_elastic_startup(
        tak_container_usage: SharedTakContainerUsage,
        capacity: ResourceCapacity,
        oversubscribe_x: u64,
        elastic_startup: ResourceCapacity,
    ) -> Self {
        Self::new_with_resource_envelope(
            tak_container_usage,
            ResourceEnvelope {
                total: capacity,
                margin: ResourceCapacity {
                    cpu_cores: 0.0,
                    memory_mb: 0,
                },
                host_reserve: ResourceCapacity {
                    cpu_cores: 0.0,
                    memory_mb: 0,
                },
                workload: capacity,
            },
            oversubscribe_x,
            elastic_startup,
            Some(HostUsageSample {
                non_tak_usage: ResourceCapacity {
                    cpu_cores: 0.0,
                    memory_mb: 0,
                },
                available_memory_mb: u64::MAX,
            }),
        )
    }
}
