use super::super::*;

mod events;
mod logs;
mod tasks;

use events::events_response;
use logs::logs_response;
use tasks::tasks_response;

pub(super) fn matches(path: &str) -> bool {
    matches!(
        path,
        "/v2/worker/status" | "/v2/worker/ping" | "/v2/worker/logs" | "/v2/worker/tasks"
    ) || task_run_id(path).is_some()
}

pub(super) fn handle(
    context: &RemoteNodeContext,
    store: &SubmitAttemptStore,
    method: &str,
    path: &str,
    query: Option<&str>,
) -> Result<WorkerHttpResponse> {
    if method != "GET" {
        return Ok(text_response(405, "method_not_allowed"));
    }
    let response = match path {
        "/v2/worker/status" => match context.node_status() {
            Ok(status) => protobuf_response(200, &status),
            Err(error) => {
                tracing::error!(%error, "failed to build worker v2 status");
                error_response(500, "status_unavailable")
            }
        },
        "/v2/worker/ping" => match context.node_ping() {
            Ok(ping) => protobuf_response(200, &ping),
            Err(error) => {
                tracing::error!(%error, "failed to build worker v2 ping");
                error_response(500, "status_unavailable")
            }
        },
        "/v2/worker/logs" => logs_response(context, query),
        "/v2/worker/tasks" => tasks_response(store, query)?,
        _ => events_response(
            store,
            task_run_id(path).expect("matched task event path"),
            query,
        )?,
    };
    Ok(wrap(response))
}

fn wrap(response: WorkerHttpResponse) -> WorkerHttpResponse {
    match tak_proto::worker_v2::encode_display_payload(&response.body) {
        Ok(body) => binary_response(response.status_code, "application/json", body),
        Err(error) => {
            tracing::error!(%error, "failed to encode worker v2 display payload");
            text_response(500, "display_payload_unavailable")
        }
    }
}

fn task_run_id(path: &str) -> Option<&str> {
    let value = path
        .strip_prefix("/v2/worker/tasks/")?
        .strip_suffix("/events")?;
    (!value.is_empty() && !value.contains('/')).then_some(value)
}
