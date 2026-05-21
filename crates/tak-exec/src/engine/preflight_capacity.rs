use anyhow::Result;
use prost::Message;
use tak_proto::NodeStatusResponse;

use super::remote_models::StrictRemoteTarget;

pub(crate) async fn remote_target_has_capacity(target: &StrictRemoteTarget) -> Result<bool> {
    let Some(required) = target_resource_limits(target) else {
        return Ok(true);
    };
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
        return Ok(true);
    }
    let status = NodeStatusResponse::decode(body.as_slice())?;
    let cpu_capacity = status
        .cpu
        .as_ref()
        .map(|cpu| f64::from(cpu.logical_cores))
        .unwrap_or(0.0);
    let memory_capacity = status
        .memory
        .as_ref()
        .map(|memory| memory.total_bytes / 1024 / 1024)
        .unwrap_or(0);
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
    Ok(used_cpu + required.cpu_cores <= cpu_capacity
        && used_memory.saturating_add(required.memory_mb) <= memory_capacity)
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
