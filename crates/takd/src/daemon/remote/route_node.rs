use super::*;
use tak_proto::CancelTaskResponse;

pub(super) fn handle_node_metadata_route(
    context: &RemoteNodeContext,
    method: &str,
    path_only: &str,
) -> Option<RemoteV1Response> {
    if method == "GET" && path_only == "/v1/node/info" {
        return Some(protobuf_response(200, &context.node));
    }
    if method == "GET" && path_only == "/v1/node/status" {
        return Some(match context.node_status() {
            Ok(status) => protobuf_response(200, &status),
            Err(err) => {
                tracing::error!("failed to build node status response: {err:#}");
                error_response(500, "status_unavailable")
            }
        });
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
        return Some(protobuf_response(
            202,
            &CancelTaskResponse {
                cancelled: true,
                task_run_id: task_run_id.to_string(),
            },
        ));
    }

    None
}
