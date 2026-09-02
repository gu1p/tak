use base64::Engine;
use tak_proto::worker_v2::{WorkerAttemptState, WorkerOutputStream};
use tak_proto::{PollTaskEventsResponse, RemoteEvent};

use super::super::super::*;

pub(super) fn events_response(
    store: &SubmitAttemptStore,
    task_run_id: &str,
    query: Option<&str>,
) -> Result<WorkerHttpResponse> {
    let after_seq = query_param_u64(query, "after_seq").unwrap_or(0);
    let attempt = query_param_u64(query, "attempt").and_then(|value| u32::try_from(value).ok());
    let Some(observation) = store.observe_worker_v2_task(task_run_id, attempt, after_seq)? else {
        return Ok(error_response(404, "task_not_found"));
    };
    let events = observation
        .events
        .into_iter()
        .map(|event| {
            let chunk_bytes =
                base64::engine::general_purpose::STANDARD.decode(&event.chunk_base64)?;
            Ok(RemoteEvent {
                seq: event.seq,
                kind: match event.stream {
                    WorkerOutputStream::Stdout => "TASK_STDOUT_CHUNK",
                    WorkerOutputStream::Stderr => "TASK_STDERR_CHUNK",
                }
                .to_string(),
                timestamp_ms: 0,
                success: None,
                exit_code: None,
                message: None,
                chunk: std::str::from_utf8(&chunk_bytes).ok().map(str::to_string),
                chunk_bytes,
                queue_position: None,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(protobuf_response(
        200,
        &PollTaskEventsResponse {
            events,
            done: matches!(
                observation.state,
                WorkerAttemptState::Completed | WorkerAttemptState::Missing
            ),
        },
    ))
}
