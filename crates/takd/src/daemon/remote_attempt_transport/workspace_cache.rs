use tak_proto::worker_v2::{
    WorkspaceCacheDisposition, WorkspaceCacheProbeRequest, WorkspaceCacheUploadRequest,
    decode_cache_response, encode_cache_probe_request, encode_cache_upload_request,
};

use super::request::PreparedDispatch;
use super::*;

pub(super) async fn ensure(
    transport: &RemoteAttemptTransport,
    target: &WorkerConnectionTarget,
    dispatch: &PreparedDispatch,
) -> Result<WorkspaceCacheDisposition> {
    let descriptor = &dispatch.request.payload.workspace.descriptor;
    let _transfer = transport
        .workspace_transfers
        .acquire(&target.node_id, &descriptor.manifest.fingerprint)
        .await?;
    let probe = WorkspaceCacheProbeRequest {
        protocol_version: 2,
        descriptor: descriptor.clone(),
    };
    let response = exchange(
        transport,
        target,
        "/v2/workspaces/cache/probe",
        &encode_cache_probe_request(&probe)?,
        &[200],
    )
    .await?;
    require_fingerprint(&response, &descriptor.manifest.fingerprint)?;
    if response.disposition == WorkspaceCacheDisposition::Hit {
        record(&dispatch.request, "hit");
        return Ok(WorkspaceCacheDisposition::Hit);
    }
    if response.disposition != WorkspaceCacheDisposition::Miss {
        bail!("worker cache probe returned an invalid disposition");
    }
    record(&dispatch.request, "miss");
    let archive = std::fs::read(&dispatch.archive_path)?;
    let upload = WorkspaceCacheUploadRequest {
        protocol_version: 2,
        descriptor: descriptor.clone(),
        archive_base64: base64::Engine::encode(&base64::engine::general_purpose::STANDARD, archive),
    };
    let response = exchange(
        transport,
        target,
        "/v2/workspaces/cache/upload",
        &encode_cache_upload_request(&upload)?,
        &[200, 201],
    )
    .await?;
    require_fingerprint(&response, &descriptor.manifest.fingerprint)?;
    if !matches!(
        response.disposition,
        WorkspaceCacheDisposition::Hit | WorkspaceCacheDisposition::Stored
    ) {
        bail!("worker cache upload did not publish the requested blob");
    }
    Ok(WorkspaceCacheDisposition::Miss)
}

fn require_fingerprint(
    response: &tak_proto::worker_v2::WorkspaceCacheResponse,
    expected: &str,
) -> Result<()> {
    if response.workspace_fingerprint != expected {
        bail!("worker cache response fingerprint mismatch");
    }
    Ok(())
}

async fn exchange(
    transport: &RemoteAttemptTransport,
    target: &WorkerConnectionTarget,
    path: &str,
    body: &[u8],
    allowed: &[u16],
) -> Result<tak_proto::worker_v2::WorkspaceCacheResponse> {
    let response = transport
        .broker
        .worker_v2_http_exchange(target, "POST", path, body)
        .await?;
    require_status(response.status, allowed, "workspace cache")?;
    let response = decode_cache_response(&response.body)?;
    Ok(response)
}

fn record(request: &tak_proto::worker_v2::DispatchAttemptRequest, disposition: &str) {
    tracing::info!(
        run_id = %request.identity.run_id,
        job_id = %request.identity.job_id,
        node_id = %request.identity.node_id,
        workspace_fingerprint = %request.payload.workspace.descriptor.manifest.fingerprint,
        cache = disposition,
        "worker workspace cache probe"
    );
}
