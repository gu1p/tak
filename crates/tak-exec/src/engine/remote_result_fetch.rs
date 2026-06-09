//! Shared resilience for fetching a remote task's result and resuming its event
//! stream: a status classifier, a rich error formatter, and a bounded
//! retry-with-backoff wrapper around the (idempotent, read-only) result GET.
//!
//! ```no_run
//! # // Reason: This behavior depends on live remote nodes and is compile-checked only.
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! #     Ok(())
//! # }
//! ```
use std::time::Duration;

use anyhow::{Result, bail};
use prost::Message;
use tak_core::model::{BackoffDef, TaskLabel};
use tak_proto::ErrorResponse;

use crate::retry::retry_backoff_delay;

use super::output_observer::emit_task_status_message;
use super::protocol_result_http::{parse_remote_protocol_result, raw_remote_protocol_result};
use super::remote_models::RemoteProtocolResult;
use super::{RemoteHttpExchangeError, StrictRemoteTarget, TaskOutputObserver, TaskStatusPhase};

/// How many times a transient (5xx / retryable transport) result fetch is retried
/// before the failure is surfaced. Deliberately tighter than the event stream's
/// 30 reconnects — a terminal GET should not stay patient for a hard 500.
const RESULT_FETCH_MAX_ATTEMPTS: u32 = 5;
/// How many times a post-`done` 404 is tolerated before declaring the result
/// genuinely missing. Covers the (non-atomic) window where the terminal event is
/// appended but the result row write raced or failed.
const RESULT_NOT_FOUND_GRACE_ATTEMPTS: u32 = 5;
/// Short fixed delay between 404 grace attempts (~1.25s total over the budget).
const RESULT_NOT_FOUND_BACKOFF: Duration = Duration::from_millis(250);

/// Exponential backoff for transient result fetches: 0.25s, 0.5s, 1s, 2s, 4s.
fn result_fetch_backoff() -> BackoffDef {
    BackoffDef::ExpJitter {
        min_s: 0.25,
        max_s: 4.0,
        jitter: String::from("none"),
    }
}

/// Classification of an HTTP status returned by a result/events fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FetchOutcome {
    /// 200 — a usable response body.
    Ok,
    /// Transient by HTTP semantics (5xx, 408, 429) — safe to retry/resume.
    Retryable,
    /// 404 — result not (yet) present.
    NotFound,
    /// Any other non-200 (ordinary 4xx, 3xx) — fail fast, retrying won't help.
    Terminal,
}

/// Maps an HTTP status to a retry decision. Status codes already encode
/// retryability, so the client classifies here rather than relying on a server
/// flag (mirrors the existing `broker_error_response` status classification).
pub(crate) fn classify_fetch_status(status: u16) -> FetchOutcome {
    match status {
        200 => FetchOutcome::Ok,
        404 => FetchOutcome::NotFound,
        408 | 429 | 500..=599 => FetchOutcome::Retryable,
        _ => FetchOutcome::Terminal,
    }
}

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

/// Tunable bounds for [`fetch_remote_result_with_policy`]. Production uses
/// [`ResultFetchPolicy::production`]; tests inject a zero-backoff policy.
pub(crate) struct ResultFetchPolicy {
    /// Max transient (5xx / retryable transport) retries before failing.
    pub(crate) max_attempts: u32,
    /// Max post-`done` 404 retries before declaring the result missing.
    pub(crate) not_found_grace: u32,
    /// Backoff for transient retries.
    pub(crate) backoff: BackoffDef,
    /// Fixed delay between 404 grace retries.
    pub(crate) not_found_backoff: Duration,
}

impl ResultFetchPolicy {
    pub(crate) fn production() -> Self {
        Self {
            max_attempts: RESULT_FETCH_MAX_ATTEMPTS,
            not_found_grace: RESULT_NOT_FOUND_GRACE_ATTEMPTS,
            backoff: result_fetch_backoff(),
            not_found_backoff: RESULT_NOT_FOUND_BACKOFF,
        }
    }
}

/// Fetches a remote task's result, retrying transient failures with backoff
/// before giving up with a rich error. The result GET is read-only and
/// idempotent, so retrying never re-runs the task or duplicates output.
///
/// - 200 → parsed result.
/// - 5xx / 408 / 429 / retryable transport error → bounded retry.
/// - 404 → bounded grace retry, since the terminal event and the result row are
///   persisted non-atomically.
/// - other 4xx / non-retryable transport error → fail immediately.
pub(crate) async fn fetch_remote_result_with_retry(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
    task_label: &TaskLabel,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
) -> Result<RemoteProtocolResult> {
    fetch_remote_result_with_policy(
        target,
        task_run_id,
        attempt,
        task_label,
        output_observer,
        &ResultFetchPolicy::production(),
    )
    .await
}

pub(crate) async fn fetch_remote_result_with_policy(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    attempt: u32,
    task_label: &TaskLabel,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    policy: &ResultFetchPolicy,
) -> Result<RemoteProtocolResult> {
    let path = format!("/v1/tasks/{task_run_id}/result");
    let mut retry_attempt = 0_u32;
    let mut not_found_attempt = 0_u32;
    loop {
        match raw_remote_protocol_result(target, task_run_id, attempt).await {
            Ok((status, body)) => match classify_fetch_status(status) {
                FetchOutcome::Ok => return parse_remote_protocol_result(target, &body),
                FetchOutcome::Retryable => {
                    retry_attempt += 1;
                    if retry_attempt > policy.max_attempts {
                        bail!(
                            "{}",
                            format_remote_fetch_failure(&RemoteFetchFailure {
                                target,
                                task_run_id,
                                attempt,
                                phase: "result",
                                path: &path,
                                status: Some(status),
                                body: Some(&body),
                                transport_error: None,
                            })
                        );
                    }
                    emit_result_retry_status(
                        output_observer,
                        task_label,
                        attempt,
                        target,
                        format!(
                            "retrying result fetch after transient HTTP {status} ({retry_attempt}/{})",
                            policy.max_attempts
                        ),
                    )?;
                    sleep_if_nonzero(retry_backoff_delay(&policy.backoff, retry_attempt)).await;
                }
                FetchOutcome::NotFound => {
                    not_found_attempt += 1;
                    if not_found_attempt > policy.not_found_grace {
                        let mut message = format_remote_fetch_failure(&RemoteFetchFailure {
                            target,
                            task_run_id,
                            attempt,
                            phase: "result",
                            path: &path,
                            status: Some(404),
                            body: Some(&body),
                            transport_error: None,
                        });
                        message.push_str(
                            "\ndiagnostic: terminal event observed but result still missing after retries; the remote worker may have failed to persist the result",
                        );
                        bail!("{message}");
                    }
                    emit_result_retry_status(
                        output_observer,
                        task_label,
                        attempt,
                        target,
                        format!(
                            "result not yet available, retrying ({not_found_attempt}/{})",
                            policy.not_found_grace
                        ),
                    )?;
                    sleep_if_nonzero(policy.not_found_backoff).await;
                }
                FetchOutcome::Terminal => {
                    bail!(
                        "{}",
                        format_remote_fetch_failure(&RemoteFetchFailure {
                            target,
                            task_run_id,
                            attempt,
                            phase: "result",
                            path: &path,
                            status: Some(status),
                            body: Some(&body),
                            transport_error: None,
                        })
                    );
                }
            },
            Err(err) if err.is_retryable() => {
                retry_attempt += 1;
                if retry_attempt > policy.max_attempts {
                    bail!(
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
                    );
                }
                emit_result_retry_status(
                    output_observer,
                    task_label,
                    attempt,
                    target,
                    format!(
                        "retrying result fetch after transient transport error ({retry_attempt}/{})",
                        policy.max_attempts
                    ),
                )?;
                sleep_if_nonzero(retry_backoff_delay(&policy.backoff, retry_attempt)).await;
            }
            Err(err) => {
                bail!(
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
                );
            }
        }
    }
}

async fn sleep_if_nonzero(wait: Duration) {
    if !wait.is_zero() {
        tokio::time::sleep(wait).await;
    }
}

fn emit_result_retry_status(
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
    task_label: &TaskLabel,
    attempt: u32,
    target: &StrictRemoteTarget,
    message: String,
) -> Result<()> {
    emit_task_status_message(
        output_observer,
        task_label,
        attempt,
        TaskStatusPhase::RetryWait,
        Some(target.node_id.as_str()),
        message,
    )
}

#[cfg(test)]
mod tests {
    use super::{FetchOutcome, classify_fetch_status, decode_error_detail};
    use prost::Message;
    use tak_proto::ErrorResponse;

    #[test]
    fn classify_fetch_status_maps_expected_outcomes() {
        assert_eq!(classify_fetch_status(200), FetchOutcome::Ok);
        assert_eq!(classify_fetch_status(404), FetchOutcome::NotFound);
        for status in [408, 429, 500, 502, 503, 599] {
            assert_eq!(
                classify_fetch_status(status),
                FetchOutcome::Retryable,
                "status {status} should be retryable"
            );
        }
        for status in [400, 401, 403, 418, 451, 301] {
            assert_eq!(
                classify_fetch_status(status),
                FetchOutcome::Terminal,
                "status {status} should be terminal"
            );
        }
    }

    #[test]
    fn decode_error_detail_prefers_protobuf_message() {
        let body = ErrorResponse {
            message: "request_failed: database is locked".to_string(),
        }
        .encode_to_vec();
        assert_eq!(
            decode_error_detail(Some(&body)),
            "request_failed: database is locked"
        );
    }

    #[test]
    fn decode_error_detail_falls_back_for_non_protobuf() {
        assert_eq!(decode_error_detail(None), "<no body>");
        // A plainly invalid protobuf byte sequence yields the preview fallback.
        let detail = decode_error_detail(Some(&[0xff, 0xff, 0xff, 0xff]));
        assert!(detail.starts_with("<4 bytes;"), "got: {detail}");
    }
}
