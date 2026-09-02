use std::num::NonZeroU32;

use tak_core::model::ContainerResourceLimitsSpec;
use tak_proto::ContainerResourceLimits;

#[derive(Debug, Clone)]
pub(crate) struct ResourceRequest {
    pub(crate) idempotency_key: String,
    pub(crate) task_run_id: String,
    pub(crate) attempt: u32,
    pub(crate) task_label: String,
    pub(crate) queued_at_ms: i64,
    pub(crate) resource_limits: ContainerResourceLimitsSpec,
    pub(crate) runtime: Option<String>,
    pub(crate) origin: Option<String>,
    pub(crate) runtime_source: Option<String>,
    pub(crate) command: Option<String>,
    pub(crate) execution_label: Option<String>,
    pub(crate) execution_slots: NonZeroU32,
}

pub(crate) fn proto_resource_limits(
    limits: &ContainerResourceLimitsSpec,
) -> Option<ContainerResourceLimits> {
    Some(ContainerResourceLimits {
        cpu_cores: limits.cpu_cores?,
        memory_mb: limits.memory_mb?,
    })
}
