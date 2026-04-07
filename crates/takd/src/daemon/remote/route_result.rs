use super::*;
use tak_proto::{GetTaskResultResponse, OutputFile};

pub(super) fn handle_remote_result_route(
    store: &SubmitAttemptStore,
    method: &str,
    path_only: &str,
    query: Option<&str>,
) -> Result<Option<RemoteV1Response>> {
    let Some(task_run_id) = remote_task_path_arg(path_only, "/result") else {
        return Ok(None);
    };
    if method != "GET" {
        return Ok(None);
    }

    let key = resolve_submit_idempotency_key_for_task_run(store, task_run_id, query)?;
    let Some(key) = key else {
        return Ok(Some(error_response(404, "task_not_found")));
    };
    let Some(payload_json) = store.result_payload(&key)? else {
        return Ok(Some(error_response(404, "result_not_found")));
    };

    let payload_value = serde_json::from_str::<serde_json::Value>(&payload_json)
        .unwrap_or_else(|_| serde_json::json!({}));
    let success = payload_value
        .get("success")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let status = if success { "success" } else { "failure" };
    let node_id = store
        .selected_node_id_for_submit(&key)?
        .unwrap_or_else(|| "unknown".to_string());
    let outputs = payload_value
        .get("outputs")
        .and_then(serde_json::Value::as_array)
        .map(|outputs| {
            outputs
                .iter()
                .map(|output| OutputFile {
                    path: output
                        .get("path")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    digest: output
                        .get("digest")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    size_bytes: output
                        .get("size")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0),
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Ok(Some(protobuf_response(
        200,
        &GetTaskResultResponse {
            success,
            exit_code: payload_value
                .get("exit_code")
                .and_then(serde_json::Value::as_i64)
                .and_then(|value| i32::try_from(value).ok()),
            status: status.to_string(),
            started_at: payload_value
                .get("started_at")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0),
            finished_at: payload_value
                .get("finished_at")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0),
            duration_ms: payload_value
                .get("duration_ms")
                .and_then(serde_json::Value::as_i64)
                .unwrap_or(0),
            node_id,
            transport_kind: payload_value
                .get("transport_kind")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("direct")
                .to_string(),
            runtime: payload_value
                .get("runtime")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            runtime_engine: payload_value
                .get("runtime_engine")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            outputs,
            stdout_tail: payload_value
                .get("stdout_tail")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
            stderr_tail: payload_value
                .get("stderr_tail")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string),
        },
    )))
}
