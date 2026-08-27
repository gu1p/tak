use std::fs;

use super::*;

pub(super) fn handle_node_logs_route(
    context: &RemoteNodeContext,
    method: &str,
    path_only: &str,
    query: Option<&str>,
) -> Option<RemoteV1Response> {
    if method != "GET" || path_only != "/v1/node/logs" {
        return None;
    }
    let Some(state_root) = context.state_root() else {
        return Some(error_response(404, "service_log_not_available"));
    };
    let log_path = state_root.join("service.log");
    let all = query_param_string(query, "all").as_deref() == Some("true");
    let contents = match if all {
        fs::read_to_string(&log_path)
    } else {
        let lines = query_param_u64(query, "lines").unwrap_or(200);
        crate::log_tail::read_log_tail(&log_path, usize::try_from(lines).unwrap_or(usize::MAX))
    } {
        Ok(contents) => contents,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Some(error_response(404, "service_log_not_found"));
        }
        Err(err) => {
            tracing::error!(
                "failed to read remote service log {}: {err}",
                log_path.display()
            );
            return Some(error_response(500, "service_log_unavailable"));
        }
    };
    Some(text_response(200, contents))
}
