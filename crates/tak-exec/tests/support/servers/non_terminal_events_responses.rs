#![allow(dead_code)]

use tak_proto::{
    ErrorResponse, GetTaskResultResponse, NodeInfo, PollTaskEventsResponse, RemoteEvent,
    SubmitTaskResponse,
};

pub fn error(message: &str) -> ErrorResponse {
    ErrorResponse {
        message: message.into(),
    }
}

pub fn node_info(port: u16) -> NodeInfo {
    NodeInfo {
        node_id: "builder-non-terminal".into(),
        display_name: "builder-non-terminal".into(),
        base_url: format!("http://127.0.0.1:{port}"),
        healthy: true,
        pools: vec!["build".into()],
        tags: vec!["builder".into()],
        capabilities: vec!["linux".into()],
        transport: "direct".into(),
    }
}

pub fn submit_response() -> SubmitTaskResponse {
    SubmitTaskResponse {
        accepted: true,
        attached: false,
        idempotency_key: "task-run-1:1".into(),
        remote_worker: true,
    }
}

pub fn event_response(call: usize) -> PollTaskEventsResponse {
    let events = if call == 1 {
        vec![RemoteEvent {
            seq: 1,
            kind: "TASK_LOG_CHUNK".into(),
            timestamp_ms: 1,
            success: None,
            exit_code: None,
            message: None,
            chunk: Some("pending\n".into()),
            chunk_bytes: Vec::new(),
        }]
    } else {
        Vec::new()
    };
    PollTaskEventsResponse {
        events,
        done: false,
    }
}

pub fn result_response() -> GetTaskResultResponse {
    GetTaskResultResponse {
        success: true,
        exit_code: Some(0),
        status: "success".into(),
        started_at: 0,
        finished_at: 0,
        duration_ms: 0,
        node_id: "builder-non-terminal".into(),
        transport_kind: "direct".into(),
        runtime: None,
        runtime_engine: None,
        outputs: Vec::new(),
        stdout_tail: None,
        stderr_tail: None,
    }
}
