use prost::Message;
use tak_proto::{GetTaskResultResponse, PollTaskEventsResponse, SubmitTaskResponse};

pub(super) mod stream;

pub(super) fn peers() -> serde_json::Value {
    serde_json::json!({
        "type": "PeersSnapshot",
        "peers": [{"node_id": "builder-retry", "endpoint": "http://builder-retry.onion"}]
    })
}

pub(super) fn upload_status(upload_id: &str, offset: u64, complete: bool) -> serde_json::Value {
    serde_json::json!({
        "type": "RemoteHttpResponse",
        "status": 200,
        "headers": [],
        "body": tak_proto::BeginWorkspaceUploadResponse {
            upload_id: upload_id.to_string(),
            offset,
            complete,
        }.encode_to_vec(),
        "upload_id": upload_id,
    })
}

pub(super) fn placed() -> serde_json::Value {
    serde_json::json!({
        "type": "RemotePlaced",
        "task_handle": "daemon-task-retry",
        "peer": {"node_id": "builder-retry", "endpoint": "http://builder-retry.onion"},
        "status": 200,
        "headers": [],
        "body": SubmitTaskResponse {
            accepted: true,
            attached: false,
            idempotency_key: "retry:2".into(),
            remote_worker: true,
        }.encode_to_vec(),
    })
}

pub(super) fn events() -> serde_json::Value {
    remote_response(PollTaskEventsResponse {
        events: Vec::new(),
        done: true,
    })
}

pub(super) fn result() -> serde_json::Value {
    remote_response(GetTaskResultResponse {
        success: true,
        exit_code: Some(0),
        status: "success".into(),
        started_at: 0,
        finished_at: 0,
        duration_ms: 0,
        node_id: "builder-retry".into(),
        transport_kind: "tor".into(),
        runtime: None,
        runtime_engine: None,
        outputs: Vec::new(),
        stdout_tail: None,
        stderr_tail: None,
    })
}

pub(super) fn error(message: &str) -> serde_json::Value {
    serde_json::json!({"type": "Error", "message": message, "retryable": false})
}

pub(super) fn classified_error(message: &str, code: &str, retryable: bool) -> serde_json::Value {
    serde_json::json!({
        "type": "Error",
        "message": message,
        "code": code,
        "retryable": retryable
    })
}

fn remote_response<T: Message>(message: T) -> serde_json::Value {
    serde_json::json!({"type": "RemoteHttpResponse", "status": 200, "headers": [], "body": message.encode_to_vec()})
}
