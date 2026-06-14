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
use tak_core::model::TaskLabel;

use super::output_observer::emit_task_status_message;
use super::protocol_result_http::{parse_remote_protocol_result, raw_remote_protocol_result};
use super::remote_models::RemoteProtocolResult;
use super::{RemoteHttpExchangeError, StrictRemoteTarget, TaskOutputObserver, TaskStatusPhase};
use crate::retry::retry_backoff_delay;

mod failure;
mod policy;
mod status;

pub(crate) use failure::{RemoteFetchFailure, format_remote_fetch_failure};
pub(crate) use policy::ResultFetchPolicy;
pub(crate) use status::{FetchOutcome, classify_fetch_status};

/// Fetches a remote task's result, retrying transient failures with backoff
/// before giving up with a rich error. The result GET is read-only and
/// idempotent, so retrying never re-runs the task or duplicates output.
///
/// - 200 → parsed result.
/// - 5xx / 408 / 429 / retryable transport error → bounded retry.
/// - 404 → bounded grace retry, since the terminal event and the result row are
///   persisted non-atomically.
/// - other 4xx / non-retryable transport error → fail immediately.
///
/// ```no_run
/// # // Reason: This helper performs remote result HTTP IO and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
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
    let fail = |status: Option<u16>,
                body: Option<&[u8]>,
                transport_error: Option<&RemoteHttpExchangeError>| {
        format_remote_fetch_failure(&RemoteFetchFailure {
            target,
            task_run_id,
            attempt,
            phase: "result",
            path: &path,
            status,
            body,
            transport_error,
        })
    };
    let mut retry_attempt = 0_u32;
    let mut not_found_attempt = 0_u32;
    loop {
        match raw_remote_protocol_result(target, task_run_id, attempt).await {
            Ok((status, body)) => match classify_fetch_status(status) {
                FetchOutcome::Ok => return parse_remote_protocol_result(target, &body),
                FetchOutcome::Retryable => {
                    retry_attempt += 1;
                    if retry_attempt > policy.max_attempts {
                        bail!("{}", fail(Some(status), Some(&body), None));
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
                        let mut message = fail(Some(404), Some(&body), None);
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
                    bail!("{}", fail(Some(status), Some(&body), None));
                }
            },
            Err(err) if err.is_retryable() => {
                retry_attempt += 1;
                if retry_attempt > policy.max_attempts {
                    bail!("{}", fail(None, None, Some(&err)));
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
                bail!("{}", fail(None, None, Some(&err)));
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
