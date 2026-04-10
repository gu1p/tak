#![allow(dead_code)]

use tak_proto::{
    GetTaskResultResponse, NodeInfo, PollTaskEventsResponse, RemoteEvent, SubmitTaskResponse,
};

pub fn shutdown_response() -> SubmitTaskResponse {
    SubmitTaskResponse {
        accepted: true,
        attached: false,
        idempotency_key: "shutdown".into(),
        remote_worker: true,
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

pub fn node_info(port: u16) -> NodeInfo {
    NodeInfo {
        node_id: "builder-delayed".into(),
        display_name: "builder-delayed".into(),
        base_url: format!("http://127.0.0.1:{port}"),
        healthy: true,
        pools: vec!["build".into()],
        tags: vec!["builder".into()],
        capabilities: vec!["linux".into()],
        transport: "direct".into(),
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
        node_id: "builder-delayed".into(),
        transport_kind: "direct".into(),
        runtime: None,
        runtime_engine: None,
        outputs: Vec::new(),
        stdout_tail: None,
        stderr_tail: None,
    }
}

pub fn event_response(call: usize) -> PollTaskEventsResponse {
    if call < 3 {
        PollTaskEventsResponse {
            events: vec![RemoteEvent {
                seq: 1,
                kind: "TASK_LOG_CHUNK".into(),
                timestamp_ms: 1,
                success: None,
                exit_code: None,
                message: None,
                chunk: Some("pending\n".into()),
                chunk_bytes: Vec::new(),
            }],
            done: false,
        }
    } else {
        PollTaskEventsResponse {
            events: vec![RemoteEvent {
                seq: 2,
                kind: "TASK_COMPLETED".into(),
                timestamp_ms: 2,
                success: Some(true),
                exit_code: Some(0),
                message: None,
                chunk: None,
                chunk_bytes: Vec::new(),
            }],
            done: true,
        }
    }
}
