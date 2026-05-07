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
    let contents = match fs::read_to_string(&log_path) {
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
    if query_param_string(query, "all").as_deref() == Some("true") {
        return Some(text_response(200, contents));
    }
    Some(text_response(
        200,
        tail_lines(&contents, query_param_u64(query, "lines").unwrap_or(200)),
    ))
}

fn tail_lines(contents: &str, lines: u64) -> String {
    if lines == 0 || contents.is_empty() {
        return String::new();
    }
    let lines = usize::try_from(lines).unwrap_or(usize::MAX);
    let all_lines = contents.lines().collect::<Vec<_>>();
    let start = all_lines.len().saturating_sub(lines);
    let mut tail = all_lines[start..].join("\n");
    if !tail.is_empty() && contents.ends_with('\n') {
        tail.push('\n');
    }
    tail
}
