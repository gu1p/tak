pub(crate) fn parse_remote_events_response(
    target: &StrictRemoteTarget,
    response_body: &str,
    last_seen_seq: u64,
) -> Result<ParsedRemoteEvents> {
    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(response_body)
        && is_wrapped_remote_events_payload(&parsed)
    {
        return parse_wrapped_remote_events(target, &parsed, last_seen_seq);
    }

    parse_ndjson_remote_events(target, response_body, last_seen_seq)
}

fn is_wrapped_remote_events_payload(parsed: &serde_json::Value) -> bool {
    parsed
        .as_object()
        .is_some_and(|object| object.contains_key("events") || object.contains_key("done"))
}

fn parse_wrapped_remote_events(
    target: &StrictRemoteTarget,
    parsed: &serde_json::Value,
    last_seen_seq: u64,
) -> Result<ParsedRemoteEvents> {
    let mut checkpoint = last_seen_seq;
    let mut remote_logs = Vec::new();
    let mut observed_new_log_seqs = HashSet::new();
    if let Some(events) = parsed.get("events") {
        let events = events.as_array().ok_or_else(|| {
            anyhow!(
                "infra error: remote node {} events payload must contain an array",
                target.node_id
            )
        })?;
        for event in events {
            let Some(seq) = event.get("seq").and_then(serde_json::Value::as_u64) else {
                continue;
            };
            if seq > checkpoint {
                checkpoint = seq;
            }
            if seq <= last_seen_seq || !observed_new_log_seqs.insert(seq) {
                continue;
            }

            let is_log_chunk = event
                .get("kind")
                .and_then(serde_json::Value::as_str)
                .is_some_and(|kind| kind == "TASK_LOG_CHUNK");
            if !is_log_chunk {
                continue;
            }

            let chunk = event
                .get("chunk")
                .and_then(serde_json::Value::as_str)
                .or_else(|| event.get("message").and_then(serde_json::Value::as_str))
                .unwrap_or_default();
            remote_logs.push(RemoteLogChunk {
                seq,
                chunk: chunk.to_string(),
            });
        }
    }
    remote_logs.sort_unstable_by_key(|chunk| chunk.seq);

    Ok(ParsedRemoteEvents {
        next_seq: checkpoint,
        done: parsed
            .get("done")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true),
        remote_logs,
    })
}

fn parse_ndjson_remote_events(
    target: &StrictRemoteTarget,
    response_body: &str,
    last_seen_seq: u64,
) -> Result<ParsedRemoteEvents> {
    let mut checkpoint = last_seen_seq;
    let mut remote_logs = Vec::new();
    let mut observed_new_log_seqs = HashSet::new();
    let mut done = false;

    for line in response_body
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
    {
        let event: serde_json::Value = serde_json::from_str(line).with_context(|| {
            format!(
                "infra error: remote node {} returned invalid NDJSON event line",
                target.node_id
            )
        })?;
        let Some(seq) = event.get("seq").and_then(serde_json::Value::as_u64) else {
            continue;
        };
        if seq > checkpoint {
            checkpoint = seq;
        }
        if seq <= last_seen_seq || !observed_new_log_seqs.insert(seq) {
            continue;
        }

        let event_type = event
            .get("type")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                event
                    .get("payload")
                    .and_then(|payload| payload.get("kind"))
                    .and_then(serde_json::Value::as_str)
            })
            .unwrap_or_default();
        if event_type == "TASK_LOG_CHUNK" {
            let payload = event.get("payload").unwrap_or(&serde_json::Value::Null);
            let chunk = payload
                .get("chunk")
                .and_then(serde_json::Value::as_str)
                .or_else(|| payload.get("message").and_then(serde_json::Value::as_str))
                .or_else(|| event.get("chunk").and_then(serde_json::Value::as_str))
                .unwrap_or_default();
            remote_logs.push(RemoteLogChunk {
                seq,
                chunk: chunk.to_string(),
            });
        }
        if matches!(
            event_type,
            "TASK_COMPLETED" | "TASK_FAILED" | "TASK_TERMINAL"
        ) {
            done = true;
        }
    }
    remote_logs.sort_unstable_by_key(|chunk| chunk.seq);

    Ok(ParsedRemoteEvents {
        next_seq: checkpoint,
        done,
        remote_logs,
    })
}
