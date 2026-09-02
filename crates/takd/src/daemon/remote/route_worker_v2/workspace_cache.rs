use base64::Engine;
use tak_proto::worker_v2::{
    WorkspaceCacheDisposition, WorkspaceCacheResponse, decode_cache_probe_request,
    decode_cache_upload_request, encode_cache_response,
};

use super::super::*;
use crate::daemon::remote::worker_v2_execution::{probe_workspace_cache, store_workspace_cache};

pub(super) fn handle(
    context: &RemoteNodeContext,
    method: &str,
    path: &str,
    body: &[u8],
) -> Result<WorkerHttpResponse> {
    if method != "POST" {
        return Ok(text_response(405, "method_not_allowed"));
    }
    match path {
        "/v2/workspaces/cache/probe" => probe(context, body),
        "/v2/workspaces/cache/upload" => upload(context, body),
        _ => Ok(text_response(404, "not_found")),
    }
}

fn probe(context: &RemoteNodeContext, body: &[u8]) -> Result<WorkerHttpResponse> {
    let request = match decode_cache_probe_request(body) {
        Ok(request) => request,
        Err(_) => return Ok(text_response(400, "invalid_workspace_cache_probe")),
    };
    let disposition = if probe_workspace_cache(context, &request.descriptor)? {
        WorkspaceCacheDisposition::Hit
    } else {
        WorkspaceCacheDisposition::Miss
    };
    response(200, request.descriptor.manifest.fingerprint, disposition)
}

fn upload(context: &RemoteNodeContext, body: &[u8]) -> Result<WorkerHttpResponse> {
    let request = match decode_cache_upload_request(body) {
        Ok(request) => request,
        Err(_) => return Ok(text_response(400, "invalid_workspace_cache_upload")),
    };
    let archive = base64::engine::general_purpose::STANDARD.decode(request.archive_base64)?;
    let fingerprint = request.descriptor.manifest.fingerprint.clone();
    let disposition = store_workspace_cache(context, &request.descriptor, &archive)?;
    let status = if disposition == WorkspaceCacheDisposition::Stored {
        201
    } else {
        200
    };
    response(status, fingerprint, disposition)
}

fn response(
    status: u16,
    workspace_fingerprint: String,
    disposition: WorkspaceCacheDisposition,
) -> Result<WorkerHttpResponse> {
    let body = encode_cache_response(&WorkspaceCacheResponse {
        protocol_version: 2,
        workspace_fingerprint,
        disposition,
    })?;
    Ok(binary_response(status, "application/json", body))
}
