use super::*;
use tak_proto::CancelTaskResponse;

pub(super) fn handle_node_metadata_route(
    context: &RemoteNodeContext,
    method: &str,
    path_only: &str,
) -> Option<RemoteV1Response> {
    if method == "GET" && path_only == "/v1/node/info" {
        return Some(match context.node_info() {
            Ok(node) => protobuf_response(200, &node),
            Err(err) => {
                tracing::error!("failed to build node info response: {err:#}");
                error_response(500, "status_unavailable")
            }
        });
    }
    if method == "GET" && path_only == "/v1/node/ping" {
        return Some(match context.node_ping() {
            Ok(ping) => protobuf_response(200, &ping),
            Err(err) => {
                tracing::error!("failed to build node ping response: {err:#}");
                error_response(500, "status_unavailable")
            }
        });
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
    context: &RemoteNodeContext,
    method: &str,
    path_only: &str,
    query: Option<&str>,
) -> Option<RemoteV1Response> {
    if let Some(task_run_id) = remote_task_path_arg(path_only, "/cancel")
        && method == "POST"
    {
        let attempt = query_param_u64(query, "attempt").and_then(|value| u32::try_from(value).ok());
        let cancelled = match context.cancel_active_task(task_run_id, attempt) {
            Ok(cancelled) => cancelled,
            Err(err) => {
                tracing::error!("failed to cancel remote task {task_run_id}: {err:#}");
                return Some(error_response(500, "cancel_failed"));
            }
        };
        return Some(protobuf_response(
            202,
            &CancelTaskResponse {
                cancelled,
                task_run_id: task_run_id.to_string(),
            },
        ));
    }

    None
}
