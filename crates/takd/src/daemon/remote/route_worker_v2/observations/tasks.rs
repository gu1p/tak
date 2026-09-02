use super::super::super::*;
use tak_proto::{ListTaskAttemptsResponse, TaskAttemptSummary};

pub(super) fn tasks_response(
    store: &SubmitAttemptStore,
    query: Option<&str>,
) -> Result<WorkerHttpResponse> {
    let state = query_param_string(query, "state").unwrap_or_else(|| "all".to_string());
    let active_only = state == "active";
    let limit = query_param_u64(query, "limit")
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or(50);
    let attempts = store.worker_v2_task_attempt_summaries(active_only, limit)?;
    Ok(protobuf_response(
        200,
        &ListTaskAttemptsResponse {
            attempts: attempts
                .into_iter()
                .map(|attempt| TaskAttemptSummary {
                    task_run_id: attempt.task_run_id,
                    attempt: attempt.attempt,
                    task_label: attempt.task_label,
                    execution_label: attempt.execution_label,
                    node_id: attempt.selected_node_id,
                    state: attempt.state,
                    created_at_ms: attempt.created_at_ms,
                    finished_at_ms: attempt.finished_at_ms,
                })
                .collect(),
        },
    ))
}
