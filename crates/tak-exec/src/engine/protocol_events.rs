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
) -> Result<Vec<RemoteLogChunk>> {
    const MAX_EVENT_RECONNECTS: u32 = 30;
    const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(100);
    const EVENT_RECONNECT_DELAY: Duration = Duration::from_millis(500);
    let max_event_wait = remote_events_max_wait_duration();

    let mut last_seen_seq = 0_u64;
    let mut reconnect_attempts = 0_u32;
    let mut persisted_remote_logs = Vec::new();
    let deadline = tokio::time::Instant::now() + max_event_wait;

    while tokio::time::Instant::now() < deadline {
        let path = format!("/v1/tasks/{task_run_id}/events?after_seq={last_seen_seq}");
        let response = remote_protocol_http_request(
            target,
            "GET",
            &path,
            None,
            "events",
            Duration::from_secs(10),
        )
        .await;

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

        let parsed = parse_remote_events_response(target, &response_body, last_seen_seq)?;
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
        if parsed.done {
            return Ok(persisted_remote_logs);
        }
        tokio::time::sleep(EVENT_POLL_INTERVAL).await;
    }

    bail!(
        "infra error: remote node {} events stream exceeded {}s without terminal completion",
        target.node_id,
        max_event_wait.as_secs()
    );
}
