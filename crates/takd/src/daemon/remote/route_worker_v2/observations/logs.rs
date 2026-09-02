use std::fs;

use super::super::super::*;

pub(super) fn logs_response(
    context: &RemoteNodeContext,
    query: Option<&str>,
) -> WorkerHttpResponse {
    let Some(state_root) = context.state_root() else {
        return error_response(404, "service_log_not_available");
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
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return error_response(404, "service_log_not_found");
        }
        Err(error) => {
            tracing::error!(
                "failed to read worker service log {}: {error}",
                log_path.display()
            );
            return error_response(500, "service_log_unavailable");
        }
    };
    text_response(200, contents)
}
