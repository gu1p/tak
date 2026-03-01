use super::*;

pub(super) fn handle_node_metadata_route(
    method: &str,
    path_only: &str,
) -> Option<RemoteV1Response> {
    if method == "GET" && path_only == "/v1/node/capabilities" {
        return Some(json_response(
            200,
            serde_json::json!({
                "compatible": true,
                "protocol_version": "v1",
                "remote_worker": true,
                "execution_mode": "remote_worker",
            }),
        ));
    }
    if method == "GET" && path_only == "/v1/node/status" {
        return Some(json_response(
            200,
            serde_json::json!({
                "healthy": true,
            }),
        ));
    }
    None
}

pub(super) fn handle_remote_cancel_route(
    method: &str,
    path_only: &str,
) -> Option<RemoteV1Response> {
    if let Some(task_run_id) = remote_task_path_arg(path_only, "/cancel")
        && method == "POST"
    {
        return Some(json_response(
            202,
            serde_json::json!({
                "cancelled": true,
                "task_run_id": task_run_id,
            }),
        ));
    }

    None
}
