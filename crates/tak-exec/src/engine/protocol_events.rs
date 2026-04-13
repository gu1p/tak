/// Opens the remote event stream endpoint for one attempt.
///
/// ```no_run
/// # // Reason: This behavior depends on internal state and is compile-checked only.
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #     Ok(())
/// # }
/// ```
async fn remote_protocol_events(
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

    let mut last_seen_seq = 0_u64;
    let mut reconnect_attempts = 0_u32;
    let mut persisted_remote_logs = Vec::new();
    let mut silent_since = tokio::time::Instant::now();
    let mut next_wait_heartbeat = silent_since + event_wait_heartbeat;
    emit_task_status_message(
        output_observer,
        task_label,
        attempt,
        TaskStatusPhase::RemoteWait,
        Some(target.node_id.as_str()),
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
                    let heartbeat_message = render_remote_wait_heartbeat(
                        target,
                        silent_since.elapsed().as_secs(),
                    )
                    .await;
                    emit_task_status_message(
                        output_observer,
                        task_label,
                        attempt,
                        TaskStatusPhase::RemoteWait,
                        Some(target.node_id.as_str()),
                        heartbeat_message,
                    )?;
                    next_wait_heartbeat += event_wait_heartbeat;
                }
            }
        };

        let (status, response_body) = match response {
            Ok(success) => {
                reconnect_attempts = 0;
                success
            }
            Err(err) => {
                reconnect_attempts += 1;
                if reconnect_attempts > MAX_EVENT_RECONNECTS {
                    bail!(
                        "infra error: remote node {} events stream resume failed after seq {}: {err}",
                        target.node_id,
                        last_seen_seq
                    );
                }
                tokio::time::sleep(EVENT_RECONNECT_DELAY).await;
                continue;
            }
        };

        if status != 200 {
            bail!(
                "infra error: remote node {} events stream failed with HTTP {}",
                target.node_id,
                status
            );
        }

        let previous_seq = last_seen_seq;
        let parsed = parse_remote_events_response(target, &response_body, last_seen_seq)?;
        let saw_new_activity = parsed.next_seq > previous_seq;
        last_seen_seq = parsed.next_seq;
        for chunk in &parsed.remote_logs {
            emit_task_output(
                output_observer,
                task_label,
                attempt,
                chunk.stream,
                &chunk.bytes,
            )?;
        }
        persisted_remote_logs.extend(parsed.remote_logs);
        if saw_new_activity {
            silent_since = tokio::time::Instant::now();
            next_wait_heartbeat = silent_since + event_wait_heartbeat;
        }
        if parsed.done {
            return Ok((persisted_remote_logs, None));
        }
        if last_seen_seq == previous_seq
            && let Some(result) = try_remote_protocol_result(target, task_run_id, attempt).await?
        {
            return Ok((persisted_remote_logs, Some(result)));
        }
        tokio::time::sleep(EVENT_POLL_INTERVAL).await;
    }
}
