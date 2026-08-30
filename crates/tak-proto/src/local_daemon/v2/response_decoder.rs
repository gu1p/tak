use serde::Deserialize;

use super::identifier::is_valid_identifier;
use super::{DaemonErrorCode, PROTOCOL_VERSION, ResponseDecodeError};

/// Maximum JSON payload size accepted from the local daemon, excluding NDJSON framing.
pub const MAX_ERROR_RESPONSE_FRAME_BYTES: usize = 64 * 1024;
pub const MAX_RESPONSE_FRAME_BYTES: usize = 64 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireErrorResponse {
    protocol_version: u64,
    #[serde(rename = "type")]
    response_type: ResponseType,
    request_id: String,
    message: String,
    code: WireDaemonErrorCode,
    retryable: bool,
}

#[derive(Deserialize)]
enum ResponseType {
    Error,
}

#[derive(Deserialize)]
enum WireDaemonErrorCode {
    #[serde(rename = "protocol_v2_not_active")]
    ProtocolV2NotActive,
    #[serde(rename = "protocol_version_invalid")]
    ProtocolVersionInvalid,
    #[serde(rename = "protocol_version_unsupported")]
    ProtocolVersionUnsupported,
    #[serde(rename = "protocol_request_invalid")]
    ProtocolRequestInvalid,
    #[serde(rename = "idempotency_conflict")]
    IdempotencyConflict,
    #[serde(rename = "run_not_found")]
    RunNotFound,
    #[serde(rename = "workspace_invalid")]
    WorkspaceInvalid,
    #[serde(rename = "run_state_invalid")]
    RunStateInvalid,
    #[serde(rename = "internal")]
    Internal,
}

pub fn decode_response(
    raw: &[u8],
    expected_request_id: &str,
) -> Result<super::Response, ResponseDecodeError> {
    if raw.len() > MAX_RESPONSE_FRAME_BYTES
        || !is_valid_identifier(expected_request_id)
        || raw.first() != Some(&b'{')
        || raw.last() != Some(&b'}')
    {
        return Err(ResponseDecodeError::ProtocolMismatch);
    }
    if let Ok(response) = serde_json::from_slice::<super::Response>(raw) {
        let (request_id, protocol_version) = response.correlation();
        if protocol_version == PROTOCOL_VERSION
            && request_id == expected_request_id
            && is_valid_identifier(request_id)
            && valid_success(&response)
        {
            return Ok(response);
        }
        return Err(ResponseDecodeError::ProtocolMismatch);
    }
    let code = decode_error_response(raw, expected_request_id)?;
    Ok(super::Response::Error {
        protocol_version: PROTOCOL_VERSION,
        request_id: expected_request_id.to_owned(),
        code,
    })
}

/// Decodes one delimiter-free, strictly correlated v2 daemon error frame.
///
/// ```rust
/// use tak_proto::local_daemon::v2::{DaemonErrorCode, decode_error_response};
///
/// let raw = br#"{"protocol_version":2,"type":"Error","request_id":"list","message":"inactive","code":"protocol_v2_not_active","retryable":false}"#;
/// assert_eq!(
///     decode_error_response(raw, "list")?,
///     DaemonErrorCode::ProtocolV2NotActive
/// );
/// # Ok::<(), tak_proto::local_daemon::v2::ResponseDecodeError>(())
/// ```
pub fn decode_error_response(
    raw: &[u8],
    expected_request_id: &str,
) -> Result<DaemonErrorCode, ResponseDecodeError> {
    if raw.len() > MAX_ERROR_RESPONSE_FRAME_BYTES {
        return Err(ResponseDecodeError::FrameTooLarge);
    }
    if !is_valid_identifier(expected_request_id)
        || raw.first() != Some(&b'{')
        || raw.last() != Some(&b'}')
    {
        return Err(ResponseDecodeError::ProtocolMismatch);
    }
    let response: WireErrorResponse =
        serde_json::from_slice(raw).map_err(|_| ResponseDecodeError::ProtocolMismatch)?;
    if response.protocol_version != PROTOCOL_VERSION
        || response.request_id != expected_request_id
        || !is_valid_identifier(&response.request_id)
        || response.retryable
    {
        return Err(ResponseDecodeError::ProtocolMismatch);
    }
    let _ = response.response_type;
    let _ = response.message;
    match response.code {
        WireDaemonErrorCode::ProtocolV2NotActive => Ok(DaemonErrorCode::ProtocolV2NotActive),
        WireDaemonErrorCode::ProtocolVersionInvalid => Ok(DaemonErrorCode::ProtocolVersionInvalid),
        WireDaemonErrorCode::ProtocolVersionUnsupported => {
            Ok(DaemonErrorCode::ProtocolVersionUnsupported)
        }
        WireDaemonErrorCode::ProtocolRequestInvalid => Ok(DaemonErrorCode::ProtocolRequestInvalid),
        WireDaemonErrorCode::IdempotencyConflict => Ok(DaemonErrorCode::IdempotencyConflict),
        WireDaemonErrorCode::RunNotFound => Ok(DaemonErrorCode::RunNotFound),
        WireDaemonErrorCode::WorkspaceInvalid => Ok(DaemonErrorCode::WorkspaceInvalid),
        WireDaemonErrorCode::RunStateInvalid => Ok(DaemonErrorCode::RunStateInvalid),
        WireDaemonErrorCode::Internal => Ok(DaemonErrorCode::Internal),
    }
}

fn valid_success(response: &super::Response) -> bool {
    use super::Response;
    match response {
        Response::Error { .. } => false,
        Response::RunSubmitted { run_id, .. }
        | Response::WorkspaceUploadProgress { run_id, .. }
        | Response::RunCommitted { run_id, .. }
        | Response::CancellationAccepted { run_id, .. }
        | Response::OutputManifest { run_id, .. } => is_valid_identifier(run_id),
        Response::RunEvents {
            run_id,
            events,
            next_event,
            state,
            terminal,
            ..
        } => {
            let sequences_are_valid = events.windows(2).all(|pair| pair[0].seq < pair[1].seq)
                && events.last().is_none_or(|event| event.seq == *next_event);
            is_valid_identifier(run_id) && sequences_are_valid && (!terminal || state.is_terminal())
        }
        Response::RunList { runs, .. } => runs.iter().all(|run| is_valid_identifier(&run.run_id)),
        Response::RunDetails { run, .. } => is_valid_identifier(&run.summary.run_id),
        Response::OutputChunk { artifact_id, .. } => is_valid_identifier(artifact_id),
    }
}
