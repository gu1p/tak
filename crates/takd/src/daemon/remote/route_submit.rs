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
    let selected_node_id = context.node.node_id.clone();
    let registration =
        store.register_submit(task_run_id, Some(payload.attempt), &selected_node_id)?;
    let (attached, idempotency_key) = match registration {
        SubmitRegistration::Created { idempotency_key } => (false, idempotency_key),
        SubmitRegistration::Attached { idempotency_key } => (true, idempotency_key),
    };

    if !attached {
        spawn_remote_worker_submit_execution(
            store.clone(),
            idempotency_key.clone(),
            selected_node_id,
            context.node.transport.clone(),
            worker_payload,
        );
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
