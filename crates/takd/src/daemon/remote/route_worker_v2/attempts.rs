use base64::Engine;
use sha2::{Digest, Sha256};
use tak_proto::worker_v2::{
    CancelAttemptResponse, DispatchAttemptResponse, DispatchDisposition, OutputChunkResponse,
    decode_cancel_request, decode_dispatch_request, decode_observe_request,
    decode_output_chunk_request, encode_cancel_response, encode_dispatch_response,
    encode_observe_response, encode_output_chunk_response,
};

use super::super::*;
use crate::daemon::remote::worker_v2_execution::pin_workspace_cache;

#[path = "attempts/acknowledgement.rs"]
mod acknowledgement;
use acknowledgement::acknowledge;

pub(super) fn handle(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    path: &str,
    body: &[u8],
) -> Result<WorkerHttpResponse> {
    match path {
        "/v2/attempts/dispatch" => dispatch(context, store, body),
        "/v2/attempts/observe" => observe(context, store, body),
        "/v2/attempts/cancel" => cancel(context, store, body),
        "/v2/attempts/output-chunk" => output_chunk(context, store, body),
        "/v2/attempts/ack" => acknowledge(context, store, body),
        _ => Ok(text_response(404, "not_found")),
    }
}

fn dispatch(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    body: &[u8],
) -> Result<WorkerHttpResponse> {
    let request = match decode_dispatch_request(body) {
        Ok(request) => request,
        Err(_) => return Ok(text_response(400, "invalid_worker_dispatch")),
    };
    if foreign_node(context, &request.identity)? {
        return Ok(text_response(409, "node_identity_mismatch"));
    }
    let Some(workspace_pin) = pin_workspace_cache(context, &request.payload.workspace.descriptor)?
    else {
        return Ok(text_response(412, "workspace_cache_miss"));
    };
    let registration = match store.register_worker_v2_attempt_with(&request, || {
        reserve_worker_v2_resources(context, &request)
    }) {
        Ok(registration) => registration,
        Err(error) if error.to_string().contains("conflicting worker dispatch") => {
            return Ok(text_response(409, "conflicting_worker_dispatch"));
        }
        Err(error) => return Err(error),
    };
    let Some((disposition, admission)) = registration else {
        return Ok(text_response(429, "worker_capacity_unavailable"));
    };
    let status = if disposition == DispatchDisposition::Accepted {
        spawn_worker_v2_execution(
            context.clone(),
            store.clone(),
            request.clone(),
            admission.expect("accepted worker dispatch has an admission lease"),
            workspace_pin,
        );
        202
    } else {
        200
    };
    let response = DispatchAttemptResponse {
        protocol_version: 2,
        fencing_token: request.identity.fencing_token,
        disposition,
    };
    Ok(binary_response(
        status,
        "application/json",
        encode_dispatch_response(&response)?,
    ))
}

fn observe(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    body: &[u8],
) -> Result<WorkerHttpResponse> {
    let request = match decode_observe_request(body) {
        Ok(request) => request,
        Err(_) => return Ok(text_response(400, "invalid_worker_observation")),
    };
    if foreign_node(context, &request.identity)? {
        return Ok(text_response(409, "node_identity_mismatch"));
    }
    let response = store.observe_worker_v2_attempt(&request.identity, request.after_event)?;
    Ok(binary_response(
        200,
        "application/json",
        encode_observe_response(&response)?,
    ))
}

fn cancel(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    body: &[u8],
) -> Result<WorkerHttpResponse> {
    let request = match decode_cancel_request(body) {
        Ok(request) => request,
        Err(_) => return Ok(text_response(400, "invalid_worker_cancellation")),
    };
    if foreign_node(context, &request.identity)? {
        return Ok(text_response(409, "node_identity_mismatch"));
    }
    let disposition = store.cancel_worker_v2_attempt(&request.identity)?;
    let status = if disposition == tak_proto::worker_v2::CancelDisposition::Requested {
        202
    } else {
        200
    };
    let response = CancelAttemptResponse {
        protocol_version: 2,
        fencing_token: request.identity.fencing_token,
        disposition,
    };
    Ok(binary_response(
        status,
        "application/json",
        encode_cancel_response(&response)?,
    ))
}

fn output_chunk(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    body: &[u8],
) -> Result<WorkerHttpResponse> {
    let request = match decode_output_chunk_request(body) {
        Ok(request) => request,
        Err(_) => return Ok(text_response(400, "invalid_worker_output_chunk")),
    };
    if foreign_node(context, &request.identity)? {
        return Ok(text_response(409, "node_identity_mismatch"));
    }
    let (chunk, eof) = store.worker_v2_output_chunk_with_eof(
        &request.identity,
        &request.artifact_id,
        request.offset,
        request.max_bytes as usize,
    )?;
    let response = OutputChunkResponse {
        protocol_version: 2,
        fencing_token: request.identity.fencing_token,
        artifact_id: request.artifact_id,
        offset: request.offset,
        chunk_base64: base64::engine::general_purpose::STANDARD.encode(&chunk),
        chunk_sha256: format!("{:x}", Sha256::digest(&chunk)),
        eof,
    };
    Ok(binary_response(
        200,
        "application/json",
        encode_output_chunk_response(&response)?,
    ))
}

fn foreign_node(
    context: &RemoteNodeContext,
    identity: &tak_proto::worker_v2::WorkerAttemptIdentity,
) -> Result<bool> {
    Ok(context.node_info()?.node_id != identity.node_id)
}
