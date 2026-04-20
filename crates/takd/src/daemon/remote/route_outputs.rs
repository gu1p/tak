use super::*;

pub(super) fn handle_remote_outputs_route(
    context: &RemoteNodeContext,
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
        return Ok(Some(error_response(404, "task_not_found")));
    };
    let Some(raw_path) = query_param_string(query, "path") else {
        return Ok(Some(error_response(400, "missing_output_path")));
    };
    let normalized = match normalize_path_ref("workspace", &raw_path) {
        Ok(path_ref) if path_ref.path != "." => path_ref.path,
        _ => return Ok(Some(error_response(400, "invalid_output_path"))),
    };
    let execution_root_base = store
        .execution_root_base_for_submit(&key)?
        .unwrap_or_else(|| remote_execution_root_base(context));
    let artifact_root = artifact_root_for_submit_key_at_base(&key, &execution_root_base);
    if artifact_root.exists() {
        let Some(bytes) = read_staged_remote_output(&artifact_root, &normalized)? else {
            return Ok(Some(error_response(404, "output_not_found")));
        };
        return Ok(Some(binary_response(
            200,
            "application/octet-stream",
            bytes,
        )));
    }

    let execution_root = execution_root_for_submit_key_at_base(&key, &execution_root_base);
    let output_path = execution_root.join(&normalized);
    let Ok(bytes) = fs::read(&output_path) else {
        return Ok(Some(error_response(404, "output_not_found")));
    };

    Ok(Some(binary_response(
        200,
        "application/octet-stream",
        bytes,
    )))
}
