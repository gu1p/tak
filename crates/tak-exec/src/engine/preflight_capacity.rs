use anyhow::Result;
use prost::Message;
use tak_proto::NodeStatusResponse;

use super::remote_models::StrictRemoteTarget;

#[path = "preflight_capacity_tests.rs"]
mod tests;

#[derive(Debug, Clone)]
pub(crate) struct RemoteTargetLoad {
    pub(crate) status_known: bool,
    pub(crate) fits_requested_resources: bool,
    pub(crate) job_count: usize,
    pub(crate) cpu_ratio: f64,
    pub(crate) memory_ratio: f64,
}

pub(crate) async fn remote_target_has_capacity(target: &StrictRemoteTarget) -> Result<bool> {
    remote_target_load(target)
        .await
        .map(|load| load.fits_requested_resources)
}

pub(crate) async fn remote_target_load(target: &StrictRemoteTarget) -> Result<RemoteTargetLoad> {
    let (status, body) = super::protocol_result_http::remote_protocol_http_request(
        target,
        "GET",
        "/v1/node/status",
        None,
        "node status",
        std::time::Duration::from_secs(2),
    )
    .await?;
    if status != 200 {
        return Ok(RemoteTargetLoad::unknown());
    }
    let status = NodeStatusResponse::decode(body.as_slice())?;
    Ok(load_from_status(target, &status))
}

fn load_from_status(target: &StrictRemoteTarget, status: &NodeStatusResponse) -> RemoteTargetLoad {
    let required = target_resource_limits(target);
    let Some(cpu_capacity) = status
        .cpu
        .as_ref()
        .map(|cpu| f64::from(cpu.logical_cores))
        .filter(|capacity| *capacity > 0.0)
    else {
        return RemoteTargetLoad::unknown();
    };
    let Some(memory_capacity) = status
        .memory
        .as_ref()
        .map(|memory| memory.total_bytes / 1024 / 1024)
        .filter(|capacity| *capacity > 0)
    else {
        return RemoteTargetLoad::unknown();
    };
    let (used_cpu, used_memory) = status
        .active_jobs
        .iter()
        .filter_map(|job| job.resource_limits.as_ref())
        .fold((0.0, 0_u64), |(cpu, memory), limits| {
            (
                cpu + limits.cpu_cores,
                memory.saturating_add(limits.memory_mb),
            )
        });
    let fits_requested_resources = required.as_ref().is_none_or(|required| {
        used_cpu + required.cpu_cores <= cpu_capacity
            && used_memory.saturating_add(required.memory_mb) <= memory_capacity
    });
    RemoteTargetLoad {
        status_known: true,
        fits_requested_resources,
        job_count: status
            .active_jobs
            .len()
            .saturating_add(status.queued_jobs.len()),
        cpu_ratio: resource_ratio(used_cpu, cpu_capacity),
        memory_ratio: resource_ratio(used_memory as f64, memory_capacity as f64),
    }
}

fn resource_ratio(used: f64, capacity: f64) -> f64 {
    if capacity <= 0.0 {
        return 0.0;
    }
    used / capacity
}

impl RemoteTargetLoad {
    pub(crate) fn unknown() -> Self {
        Self {
            status_known: false,
            fits_requested_resources: true,
            job_count: 0,
            cpu_ratio: 0.0,
            memory_ratio: 0.0,
        }
    }
}

fn target_resource_limits(
    target: &StrictRemoteTarget,
) -> Option<tak_proto::ContainerResourceLimits> {
    let Some(tak_core::model::RemoteRuntimeSpec::Containerized {
        resource_limits: Some(limits),
        ..
    }) = target.runtime.as_ref()
    else {
        return None;
    };
    Some(tak_proto::ContainerResourceLimits {
        cpu_cores: limits.cpu_cores?,
        memory_mb: limits.memory_mb?,
    })
}
