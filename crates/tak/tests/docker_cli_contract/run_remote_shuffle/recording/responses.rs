use tak_proto::{
    CpuUsage, ErrorResponse, GetTaskResultResponse, MemoryUsage, NodeStatusResponse,
    PollTaskEventsResponse, RemoteEvent, SubmitTaskResponse,
};

use crate::support::remote_cli::node_info;

pub(super) fn status_response(node_id: &str, base_url: &str) -> NodeStatusResponse {
    NodeStatusResponse {
        node: Some(node_info(node_id, base_url, "direct")),
        sampled_at_ms: 1,
        cpu: Some(CpuUsage {
            utilization_percent: Some(0.0),
            logical_cores: 8,
            ..Default::default()
        }),
        memory: Some(MemoryUsage {
            used_bytes: 0,
            total_bytes: 8 * 1024 * 1024 * 1024,
            ..Default::default()
        }),
        storage: None,
        allocated_needs: Vec::new(),
        active_jobs: Vec::new(),
        image_cache: None,
        queued_jobs: Vec::new(),
    }
}

pub(super) fn events_response(node_id: &str) -> PollTaskEventsResponse {
    PollTaskEventsResponse {
        events: vec![stdout_event(node_id), completed_event()],
        done: true,
    }
}

fn stdout_event(node_id: &str) -> RemoteEvent {
    RemoteEvent {
        seq: 1,
        kind: "TASK_STDOUT_CHUNK".into(),
        timestamp_ms: 1,
        success: None,
        exit_code: None,
        message: None,
        chunk: Some(format!("{node_id}\n")),
        chunk_bytes: format!("{node_id}\n").into_bytes(),
    }
}

fn completed_event() -> RemoteEvent {
    RemoteEvent {
        seq: 2,
        kind: "TASK_COMPLETED".into(),
        timestamp_ms: 2,
        success: Some(true),
        exit_code: Some(0),
        message: None,
        chunk: None,
        chunk_bytes: Vec::new(),
    }
}

pub(super) fn submit_response(idempotency_key: &str) -> SubmitTaskResponse {
    SubmitTaskResponse {
        accepted: true,
        attached: false,
        idempotency_key: idempotency_key.into(),
        remote_worker: true,
    }
}

pub(super) fn success_result(node_id: &str) -> GetTaskResultResponse {
    GetTaskResultResponse {
        success: true,
        exit_code: Some(0),
        status: "success".into(),
        started_at: 0,
        finished_at: 0,
        duration_ms: 0,
        node_id: node_id.into(),
        transport_kind: "direct".into(),
        runtime: None,
        runtime_engine: None,
        outputs: Vec::new(),
        stdout_tail: None,
        stderr_tail: None,
    }
}

pub(super) fn error_response(message: &str) -> ErrorResponse {
    ErrorResponse {
        message: message.into(),
    }
}
