//! Renders a remote result/events fetch failure into one diagnosable message.

use prost::Message;
use tak_proto::ErrorResponse;

use crate::engine::{RemoteHttpExchangeError, StrictRemoteTarget};

/// Everything needed to render one consistent, diagnosable fetch failure.
pub(crate) struct RemoteFetchFailure<'a> {
    pub(crate) target: &'a StrictRemoteTarget,
    pub(crate) task_run_id: &'a str,
    pub(crate) attempt: u32,
    /// "result" or "events".
    pub(crate) phase: &'a str,
    pub(crate) path: &'a str,
    /// HTTP status, or `None` for a transport-level failure (no status).
    pub(crate) status: Option<u16>,
    /// Response body, when one was received (decoded for the `remote_detail` line).
    pub(crate) body: Option<&'a [u8]>,
    /// Transport error, when the failure was below the HTTP status layer.
    pub(crate) transport_error: Option<&'a RemoteHttpExchangeError>,
}

/// Builds a multi-line, actionable error string. The previous messages carried
/// only the node id and status code; this surfaces endpoint, transport, task and
/// attempt identity, the exact path, and the decoded server detail (or a bounded
/// body preview), so a fatal remote fetch is debuggable from the client output
/// alone. Style mirrors `protocol_result_http/request/daemon/errors.rs`.
///
/// ```no_run
/// # // Reason: This formatter needs remote target fixtures and is covered by result-fetch tests.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) fn format_remote_fetch_failure(failure: &RemoteFetchFailure<'_>) -> String {
    let target = failure.target;
    let mut lines = vec![
        format!(
            "infra error: remote node {} {} fetch failed",
            target.node_id, failure.phase
        ),
        String::new(),
        format!("endpoint: {}", target.endpoint),
        format!("transport: {}", target.transport_kind.as_result_value()),
        format!("task_run_id: {}", failure.task_run_id),
        format!("attempt: {}", failure.attempt),
        format!("path: {}", failure.path),
        format!(
            "http_status: {}",
            failure.status.map_or_else(
                || "transport error".to_string(),
                |status| status.to_string()
            )
        ),
    ];
    if let Some(handle) = target.daemon_task_handle.as_deref() {
        lines.push(format!("daemon_task_handle: {handle}"));
    }
    let detail = match failure.transport_error {
        Some(err) => err.to_string(),
        None => decode_error_detail(failure.body),
    };
    lines.push(format!("remote_detail: {detail}"));
    lines.push(format!("source: {}:{}", file!(), line!()));
    lines.join("\n")
}

/// Decodes the server's `ErrorResponse.message`, falling back to a bounded
/// UTF-8/byte-length preview when the body is absent or not that protobuf.
///
/// ```no_run
/// # // Reason: This private decoder is exercised through result-fetch tests.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
fn decode_error_detail(body: Option<&[u8]>) -> String {
    let Some(body) = body else {
        return "<no body>".to_string();
    };
    if let Ok(parsed) = ErrorResponse::decode(body)
        && !parsed.message.is_empty()
    {
        return parsed.message;
    }
    let preview_len = body.len().min(256);
    format!(
        "<{} bytes; utf8: {:?}>",
        body.len(),
        String::from_utf8_lossy(&body[..preview_len])
    )
}

#[path = "failure_tests.rs"]
mod failure_tests;
