use std::time::Duration;

use anyhow::{Context, Result, bail};
use prost::Message;
use tak_proto::GetTaskResultResponse;

use super::remote_result_fetch::{
    FetchOutcome, RemoteFetchFailure, classify_fetch_status, format_remote_fetch_failure,
};
use super::{RemoteHttpExchangeError, StrictRemoteTarget, remote_models::RemoteProtocolResult};
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

/// Performs the raw result GET, returning the HTTP status and body verbatim and
/// surfacing transport failures as a typed `RemoteHttpExchangeError` so callers
/// can classify retryability. The result endpoint is read-only and idempotent.
pub(crate) async fn raw_remote_protocol_result(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    _attempt: u32,
) -> std::result::Result<(u16, Vec<u8>), RemoteHttpExchangeError> {
    let path = format!("/v1/tasks/{task_run_id}/result");
    remote_protocol_http_request(target, "GET", &path, None, "result", REMOTE_RESULT_TIMEOUT).await
}

/// Outcome of an opportunistic result probe issued while the event stream is
/// still polling.
pub(crate) enum ResultProbe {
    /// The result is available.
    Ready(RemoteProtocolResult),
    /// 404 — the result is not present yet (task still running).
    NotReady,
    /// A transient failure (5xx or retryable transport error). The caller keeps
    /// polling but should bound how long it tolerates this, since a *persistent*
    /// transient failure is otherwise indistinguishable from "still running".
    /// Carries the last status/body so an eventual giveup can render a rich error.
    Transient {
        status: Option<u16>,
        body: Option<Vec<u8>>,
    },
}

/// Probes for a remote result during the event poll loop. A not-ready (404) or
/// transient (5xx / retryable transport) outcome lets the caller keep polling;
/// only a terminal (non-retryable) failure aborts here, with a rich error. The
/// terminal result fetch (with bounded retry) lives in `remote_result_fetch`.
pub(crate) async fn probe_remote_protocol_result(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
) -> Result<ResultProbe> {
    let path = format!("/v1/tasks/{task_run_id}/result");
    match raw_remote_protocol_result(target, task_run_id, attempt).await {
        Ok((status, response_body)) => match classify_fetch_status(status) {
            FetchOutcome::Ok => Ok(ResultProbe::Ready(parse_remote_protocol_result(
                target,
                &response_body,
            )?)),
            FetchOutcome::NotFound => Ok(ResultProbe::NotReady),
            FetchOutcome::Retryable => Ok(ResultProbe::Transient {
                status: Some(status),
                body: Some(response_body),
            }),
            FetchOutcome::Terminal => bail!(
                "{}",
                format_remote_fetch_failure(&RemoteFetchFailure {
                    target,
                    task_run_id,
                    attempt,
                    phase: "result",
                    path: &path,
                    status: Some(status),
                    body: Some(&response_body),
                    transport_error: None,
                })
            ),
        },
        Err(err) if err.is_retryable() => Ok(ResultProbe::Transient {
            status: None,
            body: None,
        }),
        Err(err) => bail!(
            "{}",
            format_remote_fetch_failure(&RemoteFetchFailure {
                target,
                task_run_id,
                attempt,
                phase: "result",
                path: &path,
                status: None,
                body: None,
                transport_error: Some(&err),
            })
        ),
    }
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
