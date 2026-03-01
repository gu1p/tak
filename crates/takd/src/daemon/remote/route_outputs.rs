use super::*;

pub(super) fn handle_remote_outputs_route(
    store: &SubmitAttemptStore,
    method: &str,
    path_only: &str,
    query: Option<&str>,
) -> Result<Option<RemoteV1Response>> {
    let Some(task_run_id) = remote_task_path_arg(path_only, "/outputs") else {
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
    let Some(raw_path) = query_param_string(query, "path") else {
        return Ok(Some(json_response(
            400,
            serde_json::json!({"error":"missing_output_path"}),
        )));
    };
    let normalized = match normalize_path_ref("workspace", raw_path) {
        Ok(path_ref) if path_ref.path != "." => path_ref.path,
        _ => {
            return Ok(Some(json_response(
                400,
                serde_json::json!({"error":"invalid_output_path"}),
            )));
        }
    };
    let execution_root = execution_root_for_submit_key(&key);
    let output_path = execution_root.join(&normalized);
    let Ok(bytes) = fs::read(&output_path) else {
        return Ok(Some(json_response(
            404,
            serde_json::json!({"error":"output_not_found"}),
        )));
    };

    Ok(Some(json_response(
        200,
        serde_json::json!({
            "path": normalized,
            "data_base64": base64::engine::general_purpose::STANDARD.encode(bytes),
        }),
    )))
}
