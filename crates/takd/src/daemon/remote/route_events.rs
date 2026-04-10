use super::*;
use base64::Engine;
use tak_proto::{PollTaskEventsResponse, RemoteEvent};

pub(super) fn handle_remote_events_route(
    store: &SubmitAttemptStore,
    method: &str,
    path_only: &str,
    query: Option<&str>,
) -> Result<Option<RemoteV1Response>> {
    let Some(task_run_id) = remote_task_path_arg(path_only, "/events") else {
        return Ok(None);
    };
    if method != "GET" {
        return Ok(None);
    }

    let after_seq = query_param_u64(query, "after_seq").unwrap_or(0);
    let key = resolve_submit_idempotency_key_for_task_run(store, task_run_id, query)?;
    let Some(key) = key else {
        return Ok(Some(error_response(404, "task_not_found")));
    };

    let events = store.events(&key)?;
    let mut done = false;
    let mut protobuf_events = Vec::new();
    for event in events {
        let payload_value = serde_json::from_str::<serde_json::Value>(&event.payload_json)
            .unwrap_or_else(|_| serde_json::json!({ "raw": event.payload_json }));
        let kind = payload_value
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("EVENT")
            .to_string();
        done |= matches!(
            kind.as_str(),
            "TASK_COMPLETED" | "TASK_FAILED" | "TASK_TERMINAL"
        );
        if event.seq <= after_seq {
            continue;
        }
        protobuf_events.push(RemoteEvent {
            seq: event.seq,
            kind,
            timestamp_ms: payload_value
                .get("timestamp_ms")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0),
            success: payload_value
                .get("success")
                .and_then(serde_json::Value::as_bool),
            exit_code: payload_value
                .get("exit_code")
                .and_then(serde_json::Value::as_i64)
                .and_then(|value| i32::try_from(value).ok()),
            message: payload_value
                .get("message")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            chunk: payload_value
                .get("chunk")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            chunk_bytes: payload_value
                .get("chunk_base64")
                .and_then(serde_json::Value::as_str)
                .and_then(|value| base64::engine::general_purpose::STANDARD.decode(value).ok())
                .or_else(|| {
                    payload_value
                        .get("chunk")
                        .and_then(serde_json::Value::as_str)
                        .map(|value| value.as_bytes().to_vec())
                })
                .or_else(|| {
                    payload_value
                        .get("message")
                        .and_then(serde_json::Value::as_str)
                        .map(|value| value.as_bytes().to_vec())
                })
                .unwrap_or_default(),
        });
    }

    Ok(Some(protobuf_response(
        200,
        &PollTaskEventsResponse {
            events: protobuf_events,
            done,
        },
    )))
}
