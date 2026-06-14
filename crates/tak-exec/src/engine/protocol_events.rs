use std::time::Duration;

use anyhow::{Result, bail};
use tak_core::model::TaskLabel;

use super::protocol_result_http::{
    ResultProbe, probe_remote_protocol_result, remote_protocol_http_request,
};
use super::remote_models::RemoteProtocolResult;
use super::remote_result_fetch::{FetchOutcome, classify_fetch_status};
use super::remote_wait_status::{remote_wait_heartbeat_interval, render_remote_wait_heartbeat};
use super::{RemoteLogChunk, StrictRemoteTarget, TaskOutputObserver};

use crate::remote_protocol_codec::parse_remote_events_response;

mod emit;
mod failure;
use emit::{emit_event_batch, emit_remote_wait};
use failure::{EventStreamTarget, event_stream_failure};

/// Opens the remote event stream endpoint for one attempt.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
pub(crate) async fn remote_protocol_events(
    target: &StrictRemoteTarget,
    task_run_id: &str,
    task_label: &TaskLabel,
    attempt: u32,
    output_observer: Option<&std::sync::Arc<dyn TaskOutputObserver>>,
) -> Result<(Vec<RemoteLogChunk>, Option<RemoteProtocolResult>)> {
    const MAX_EVENT_RECONNECTS: u32 = 30;
    const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(100);
    const EVENT_RECONNECT_DELAY: Duration = Duration::from_millis(500);
    let event_wait_heartbeat = remote_wait_heartbeat_interval();

    let result_probe_path = format!("/v1/tasks/{task_run_id}/result");
    let failure = EventStreamTarget {
        target,
        task_run_id,
        attempt,
    };
    let mut last_seen_seq = 0_u64;
    let mut reconnect_attempts = 0_u32;
    // Bounds consecutive transient (5xx / retryable transport) result probes while
    // the stream is otherwise alive, so a *persistent* result-endpoint failure
    // eventually surfaces instead of polling forever. Reset on progress / 404.
    let mut result_probe_failures = 0_u32;
    let mut persisted_remote_logs = Vec::new();
    let mut silent_since = tokio::time::Instant::now();
    let mut next_wait_heartbeat = silent_since + event_wait_heartbeat;
    emit_remote_wait(
        output_observer,
        target,
        task_label,
        attempt,
        format!("waiting for remote output from {}", target.node_id),
    )?;

    loop {
        let path = format!("/v1/tasks/{task_run_id}/events?after_seq={last_seen_seq}");
        let response = remote_protocol_http_request(
            target,
            "GET",
            &path,
            None,
            "events",
            Duration::from_secs(10),
        );
        tokio::pin!(response);

        let response = loop {
            let wait_heartbeat = tokio::time::sleep_until(next_wait_heartbeat);
            tokio::pin!(wait_heartbeat);
            tokio::select! {
                response = &mut response => break response,
                _ = &mut wait_heartbeat => {
                    let heartbeat_message =
                        render_remote_wait_heartbeat(target, silent_since.elapsed().as_secs()).await;
                    emit_remote_wait(output_observer, target, task_label, attempt, heartbeat_message)?;
                    next_wait_heartbeat += event_wait_heartbeat;
                }
            }
        };

        let (status, response_body) = match response {
            Ok(success) => success,
            Err(err) => {
                reconnect_attempts += 1;
                if reconnect_attempts > MAX_EVENT_RECONNECTS {
                    bail!(
                        "{}",
                        event_stream_failure(&failure, "events", &path, None, None, Some(&err))
                    );
                }
                tokio::time::sleep(EVENT_RECONNECT_DELAY).await;
                continue;
            }
        };

        // Reset the reconnect budget only on a genuine 200. A non-200 is still a
        // *successful* HTTP exchange, so resetting on any `Ok` would let a hard
        // 5xx loop forever (reset -> +1 -> reset -> +1 ...) and never hit the cap.
        if status == 200 {
            reconnect_attempts = 0;
        } else {
            let retryable = classify_fetch_status(status) == FetchOutcome::Retryable;
            if retryable {
                reconnect_attempts += 1;
            }
            if !retryable || reconnect_attempts > MAX_EVENT_RECONNECTS {
                bail!(
                    "{}",
                    event_stream_failure(
                        &failure,
                        "events",
                        &path,
                        Some(status),
                        Some(&response_body),
                        None
                    )
                );
            }
            // Resume from the same cursor (`after_seq={last_seen_seq}`); the events
            // parser drops already-seen seqs, so a transient 5xx never replays output.
            tokio::time::sleep(EVENT_RECONNECT_DELAY).await;
            continue;
        }

        let previous_seq = last_seen_seq;
        let parsed = parse_remote_events_response(target, &response_body, last_seen_seq)?;
        debug_assert_eq!(parsed.status_messages.len(), parsed.status_updates.len());
        let saw_new_activity = parsed.next_seq > previous_seq;
        last_seen_seq = parsed.next_seq;
        emit_event_batch(
            output_observer,
            target,
            task_label,
            task_run_id,
            attempt,
            &parsed.status_updates,
            &parsed.remote_logs,
        )?;
        persisted_remote_logs.extend(parsed.remote_logs);
        if saw_new_activity {
            silent_since = tokio::time::Instant::now();
            next_wait_heartbeat = silent_since + event_wait_heartbeat;
            // Progress proves the stream is healthy; forgive earlier transient
            // result probes.
            result_probe_failures = 0;
        }
        if parsed.done {
            return Ok((persisted_remote_logs, None));
        }
        if last_seen_seq == previous_seq {
            match probe_remote_protocol_result(target, task_run_id, attempt).await? {
                ResultProbe::Ready(result) => {
                    return Ok((persisted_remote_logs, Some(result)));
                }
                ResultProbe::NotReady => {
                    // Result genuinely not ready yet — keep waiting on the stream.
                    result_probe_failures = 0;
                }
                ResultProbe::Transient { status, body } => {
                    // Tolerate transient result failures (don't kill a live run),
                    // but cap them so a persistent result-endpoint fault surfaces
                    // instead of polling forever.
                    result_probe_failures += 1;
                    if result_probe_failures > MAX_EVENT_RECONNECTS {
                        bail!(
                            "{}",
                            event_stream_failure(
                                &failure,
                                "result",
                                &result_probe_path,
                                status,
                                body.as_deref(),
                                None
                            )
                        );
                    }
                }
            }
        }
        tokio::time::sleep(EVENT_POLL_INTERVAL).await;
    }
}
