use std::time::Duration;

use anyhow::{Context, Result, bail};
use prost::Message;
use tak_proto::GetTaskResultResponse;

use super::{StrictRemoteTarget, remote_models::RemoteProtocolResult};
use crate::remote_protocol_codec::parse_remote_result_outputs;

#[path = "protocol_result_http/request.rs"]
mod request;

pub(crate) use request::{
    DaemonWorkspaceUploadStreamRequest, DaemonWorkspaceWormholeUploadRequest, RemoteHttpResponse,
    StreamUploadProgress, remote_protocol_http_request,
    remote_protocol_http_request_with_extra_headers, send_workspace_wormhole_via_daemon,
    stream_workspace_upload_via_daemon,
};

const REMOTE_RESULT_TIMEOUT: Duration = Duration::from_secs(10);

pub(crate) async fn remote_protocol_result(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
) -> Result<RemoteProtocolResult> {
    let Some(result) = try_remote_protocol_result(target, task_run_id, attempt).await? else {
        bail!(
            "infra error: remote node {} result fetch failed with HTTP 404",
            target.node_id
        );
    };
    Ok(result)
}

pub(crate) async fn try_remote_protocol_result(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    _attempt: u32,
) -> Result<Option<RemoteProtocolResult>> {
    let path = format!("/v1/tasks/{task_run_id}/result");
    let (status, response_body) =
        remote_protocol_http_request(target, "GET", &path, None, "result", REMOTE_RESULT_TIMEOUT)
            .await?;
    if status == 404 {
        return Ok(None);
    }
    if status != 200 {
        bail!(
            "infra error: remote node {} result fetch failed with HTTP {}",
            target.node_id,
            status
        );
    }
    Ok(Some(parse_remote_protocol_result(target, &response_body)?))
}

pub(crate) fn parse_remote_protocol_result(
    target: &StrictRemoteTarget,
    response_body: &[u8],
) -> Result<RemoteProtocolResult> {
    let parsed = GetTaskResultResponse::decode(response_body).with_context(|| {
        format!(
            "infra error: remote node {} returned invalid protobuf for result",
            target.node_id
        )
    })?;
    let synced_outputs = parse_remote_result_outputs(target, &parsed)?;
    Ok(RemoteProtocolResult {
        success: parsed.success,
        exit_code: parsed.exit_code,
        failure_detail: (!parsed.success)
            .then_some(parsed.stderr_tail.clone())
            .flatten()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty()),
        synced_outputs,
        runtime_kind: parsed.runtime,
        runtime_engine: parsed.runtime_engine,
        stdout_tail: parsed.stdout_tail,
        stderr_tail: parsed.stderr_tail,
    })
}
