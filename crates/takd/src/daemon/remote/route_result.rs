use super::*;

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
        return Ok(Some(json_response(
            404,
            serde_json::json!({"error":"task_not_found"}),
        )));
    };
    let Some(payload_json) = store.result_payload(&key)? else {
        return Ok(Some(json_response(
            404,
            serde_json::json!({"error":"result_not_found"}),
        )));
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

    Ok(Some(json_response(
        200,
        serde_json::json!({
            "success": success,
            "status": status,
            "exit_code": payload_value.get("exit_code").cloned().unwrap_or_else(|| serde_json::json!(1)),
            "started_at": payload_value.get("started_at").cloned().unwrap_or_else(|| serde_json::json!(0)),
            "finished_at": payload_value.get("finished_at").cloned().unwrap_or_else(|| serde_json::json!(0)),
            "duration_ms": payload_value.get("duration_ms").cloned().unwrap_or_else(|| serde_json::json!(0)),
            "node_id": node_id,
            "transport_kind": payload_value.get("transport_kind").cloned().unwrap_or_else(|| serde_json::json!("direct")),
            "runtime": payload_value.get("runtime").cloned().unwrap_or(serde_json::Value::Null),
            "runtime_engine": payload_value.get("runtime_engine").cloned().unwrap_or(serde_json::Value::Null),
            "log_artifact_uri": payload_value.get("log_artifact_uri").cloned().unwrap_or(serde_json::Value::Null),
            "outputs": payload_value.get("outputs").cloned().unwrap_or_else(|| serde_json::json!([])),
            "stdout_tail": payload_value.get("stdout_tail").cloned().unwrap_or(serde_json::Value::Null),
            "stderr_tail": payload_value.get("stderr_tail").cloned().unwrap_or(serde_json::Value::Null),
        }),
    )))
}
