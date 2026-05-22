use anyhow::{Result, anyhow};
use tak_core::model::{ContainerResourceLimitsSpec, RemoteRuntimeSpec};
use tak_proto::ContainerResourceLimits;

use super::super::query_helpers::unix_epoch_ms;

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
}

pub(crate) struct ResourceRequestInput<'a> {
    pub(crate) idempotency_key: &'a str,
    pub(crate) task_run_id: &'a str,
    pub(crate) attempt: u32,
    pub(crate) task_label: &'a str,
    pub(crate) runtime: Option<&'a RemoteRuntimeSpec>,
    pub(crate) origin: Option<String>,
    pub(crate) runtime_source: Option<String>,
    pub(crate) command: Option<String>,
    pub(crate) execution_label: Option<String>,
}

impl ResourceRequest {
    pub(crate) fn new(input: ResourceRequestInput<'_>) -> Result<Self> {
        Ok(Self {
            idempotency_key: input.idempotency_key.to_string(),
            task_run_id: input.task_run_id.to_string(),
            attempt: input.attempt,
            task_label: input.task_label.to_string(),
            queued_at_ms: unix_epoch_ms(),
            resource_limits: resource_limits_from_runtime(input.runtime)?,
            runtime: input.runtime.map(|_| "containerized".to_string()),
            origin: input.origin,
            runtime_source: input.runtime_source,
            command: input.command,
            execution_label: input.execution_label,
        })
    }
}

pub(crate) fn proto_resource_limits(
    limits: &ContainerResourceLimitsSpec,
) -> Option<ContainerResourceLimits> {
    Some(ContainerResourceLimits {
        cpu_cores: limits.cpu_cores?,
        memory_mb: limits.memory_mb?,
    })
}

fn resource_limits_from_runtime(
    runtime: Option<&RemoteRuntimeSpec>,
) -> Result<ContainerResourceLimitsSpec> {
    let Some(RemoteRuntimeSpec::Containerized {
        resource_limits: Some(limits),
        ..
    }) = runtime
    else {
        return Err(anyhow!(
            "invalid_submit_fields: execution.runtime.container.resource_limits is required"
        ));
    };
    if limits.cpu_cores.is_none() || limits.memory_mb.is_none() {
        return Err(anyhow!(
            "invalid_submit_fields: execution.runtime.container.resource_limits is required"
        ));
    }
    Ok(limits.clone())
}
