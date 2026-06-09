use super::resource_admission::{ResourceAdmissionDecision, ResourceRequest, ResourceRequestInput};
use super::*;
use prost::Message;
use tak_proto::{SubmitTaskRequest, SubmitTaskResponse};

pub(super) fn handle_remote_submit_route(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    body: Option<&[u8]>,
) -> Result<RemoteV1Response> {
    let Some(body) = body else {
        return Ok(error_response(400, "missing_body"));
    };
    let payload = match SubmitTaskRequest::decode(body) {
        Ok(value) => value,
        Err(_) => return Ok(error_response(400, "invalid_protobuf")),
    };
    let task_run_id = payload.task_run_id.trim();
    if task_run_id.is_empty() || payload.attempt == 0 {
        return Ok(error_response(400, "invalid_submit_fields"));
    }

    let worker_payload = match parse_remote_worker_submit_payload(context, &payload) {
        Ok(worker_payload) => worker_payload,
        Err(err) => {
            // A referenced workspace upload that was reaped (NotFound) is reported as a
            // distinct, retryable status so the client can transparently re-upload and
            // resubmit, instead of treating it like a malformed request.
            if err
                .chain()
                .any(|cause| cause.is::<WorkspaceUploadMissing>())
            {
                return Ok(error_response(409, "workspace_upload_missing"));
            }
            return Ok(error_response(400, "invalid_submit_fields"));
        }
    };
    tracing::info!(
        task_run_id,
        attempt = payload.attempt,
        task_label = %payload.task_label,
        body_bytes = body.len(),
        workspace_bytes = worker_payload.workspace_zip.len(),
        "remote submit received"
    );
    let execution_root_base =
        ensure_remote_execution_root_base(context, worker_payload.runtime.as_ref());
    let node = context.node_info()?;
    let selected_node_id = node.node_id.clone();
    let registration = store.register_submit_with_execution_root_base(
        task_run_id,
        Some(payload.attempt),
        &payload.task_label,
        worker_payload.execution_label.as_deref(),
        &selected_node_id,
        &execution_root_base,
    )?;
    let (attached, idempotency_key) = match registration {
        SubmitRegistration::Created { idempotency_key } => (false, idempotency_key),
        SubmitRegistration::Attached { idempotency_key } => (true, idempotency_key),
    };
    tracing::info!(
        task_run_id,
        attempt = payload.attempt,
        task_label = %payload.task_label,
        attached,
        idempotency_key,
        "remote submit accepted"
    );

    if !attached {
        let cancellation = context.register_active_execution(
            idempotency_key.clone(),
            task_run_id,
            payload.attempt,
        )?;
        let admission = match prepare_resource_admission(context, &idempotency_key, &worker_payload)
        {
            Ok(admission) => admission,
            Err(error) => {
                let _ = context.unregister_active_execution(&idempotency_key);
                return Err(error);
            }
        };
        if let PreparedResourceAdmission::Admitted { started_at } = admission
            && let Err(error) = register_active_job(
                context,
                &idempotency_key,
                &worker_payload,
                &execution_root_base,
                started_at,
            )
        {
            let _ = context.unregister_active_execution(&idempotency_key);
            let _ = context.release_resources(&idempotency_key);
            return Err(error);
        }
        let execution = RemoteWorkerSubmitExecution {
            store: store.clone(),
            context: context.clone(),
            idempotency_key: idempotency_key.clone(),
            execution_root_base,
            selected_node_id,
            transport_kind: node.transport,
            image_cache: context.image_cache_config(),
            cancellation,
            payload: worker_payload,
            admission,
        };
        if let Err(err) = spawn_remote_worker_submit_execution(execution) {
            let _ = context.finish_active_job(&idempotency_key);
            let _ = context.unregister_active_execution(&idempotency_key);
            let _ = context.release_resources(&idempotency_key);
            return Err(err);
        }
    }

    Ok(protobuf_response(
        200,
        &SubmitTaskResponse {
            accepted: true,
            attached,
            idempotency_key,
            remote_worker: true,
        },
    ))
}

fn prepare_resource_admission(
    context: &RemoteNodeContext,
    idempotency_key: &str,
    payload: &RemoteWorkerSubmitPayload,
) -> Result<PreparedResourceAdmission> {
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
    })?;
    let queued_at_ms = request.queued_at_ms;
    match context.admit_or_queue_resources(request)? {
        ResourceAdmissionDecision::Admitted => Ok(PreparedResourceAdmission::Admitted {
            started_at: unix_epoch_ms(),
        }),
        ResourceAdmissionDecision::Queued { queue_position } => {
            Ok(PreparedResourceAdmission::Queued {
                queue_position,
                queued_at_ms,
            })
        }
        ResourceAdmissionDecision::Rejected { reason } => Err(anyhow!(
            "resource_requirements_exceed_worker_capacity: {reason}"
        )),
    }
}
