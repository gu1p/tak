use tak_core::model::ContainerResourceLimitsSpec;
use tak_proto::worker_v2::DispatchAttemptRequest;

use super::super::RemoteNodeContext;
use super::super::query_helpers::unix_epoch_ms;
use super::super::resource_admission::ResourceRequest;

pub(in crate::daemon::remote) struct WorkerV2AdmissionLease {
    context: RemoteNodeContext,
    key: String,
}

pub(in crate::daemon::remote) fn reserve_worker_v2_resources(
    context: &RemoteNodeContext,
    request: &DispatchAttemptRequest,
) -> anyhow::Result<Option<WorkerV2AdmissionLease>> {
    let key = format!("v2:{}", request.identity.fencing_token);
    let resources = request.payload.resources;
    let admission = ResourceRequest {
        idempotency_key: key.clone(),
        task_run_id: format!("{}/{}", request.identity.run_id, request.identity.job_id),
        attempt: request.identity.authored_attempt,
        task_label: request.payload.tasks[0].task_id.clone(),
        queued_at_ms: unix_epoch_ms(),
        resource_limits: ContainerResourceLimitsSpec {
            cpu_cores: Some(resources.cpu_millis as f64 / 1_000.0),
            memory_mb: Some(resources.memory_bytes.div_ceil(1024 * 1024)),
        },
        runtime: None,
        origin: Some("daemon-v2".into()),
        runtime_source: None,
        command: None,
        execution_label: None,
        execution_slots: resources.execution_slots,
    };
    if !context.resource_admission().admit_immediately(admission)? {
        return Ok(None);
    }
    Ok(Some(WorkerV2AdmissionLease {
        context: context.clone(),
        key,
    }))
}

impl Drop for WorkerV2AdmissionLease {
    fn drop(&mut self) {
        if let Err(error) = self.context.release_resources(&self.key) {
            tracing::error!(error = %error, "failed to release worker v2 resource admission");
        }
    }
}
