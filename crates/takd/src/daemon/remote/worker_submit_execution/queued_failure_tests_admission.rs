use super::super::*;
use crate::daemon::remote::RemoteNodeContext;
use crate::daemon::remote::resource_admission::{
    ResourceAdmissionDecision, ResourceRequest, ResourceRequestInput,
};

pub(crate) fn admit_resources(
    context: &RemoteNodeContext,
    idempotency_key: &str,
    payload: &RemoteWorkerSubmitPayload,
) {
    let request = ResourceRequest::new(ResourceRequestInput {
        idempotency_key,
        task_run_id: &payload.task_run_id,
        attempt: payload.attempt,
        task_label: &payload.task_label,
        runtime: payload.runtime.as_ref(),
        origin: payload.origin.clone(),
        runtime_source: payload.runtime_source.clone(),
        command: payload.command.clone(),
        execution_label: payload.execution_label.clone(),
    })
    .expect("resource request");
    assert!(matches!(
        context.admit_or_queue_resources(request).unwrap(),
        ResourceAdmissionDecision::Admitted
    ));
}
