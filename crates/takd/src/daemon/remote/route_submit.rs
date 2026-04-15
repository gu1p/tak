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

    let worker_payload = match parse_remote_worker_submit_payload(&payload) {
        Ok(worker_payload) => worker_payload,
        Err(_) => return Ok(error_response(400, "invalid_submit_fields")),
    };
    let execution_root_base = ensure_remote_execution_root_base(worker_payload.runtime.as_ref());
    let node = context.node_info()?;
    let selected_node_id = node.node_id.clone();
    let registration = store.register_submit_with_execution_root_base(
        task_run_id,
        Some(payload.attempt),
        &selected_node_id,
        &execution_root_base,
    )?;
    let (attached, idempotency_key) = match registration {
        SubmitRegistration::Created { idempotency_key } => (false, idempotency_key),
        SubmitRegistration::Attached { idempotency_key } => (true, idempotency_key),
    };

    if !attached {
        let execution_root =
            execution_root_for_submit_key_at_base(&idempotency_key, &execution_root_base);
        context.register_active_job(
            idempotency_key.clone(),
            super::status_state::ActiveJobMetadata::new(
                task_run_id,
                payload.attempt,
                &payload.task_label,
                &payload.needs,
                payload.runtime.as_ref().and_then(runtime_label),
                execution_root,
            ),
        )?;
        if let Err(err) = spawn_remote_worker_submit_execution(
            store.clone(),
            context.shared_status_state(),
            idempotency_key.clone(),
            selected_node_id,
            node.transport,
            worker_payload,
        ) {
            let _ = context.finish_active_job(&idempotency_key);
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

fn runtime_label(runtime: &tak_proto::RuntimeSpec) -> Option<String> {
    runtime.kind.as_ref().map(|kind| match kind {
        tak_proto::runtime_spec::Kind::Container(_) => "containerized".to_string(),
    })
}
