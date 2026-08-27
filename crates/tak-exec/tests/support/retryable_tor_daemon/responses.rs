use prost::Message;
use tak_proto::{GetTaskResultResponse, PollTaskEventsResponse, SubmitTaskResponse};

pub(super) mod placement;
pub(super) mod stream;

pub(super) fn peers(failover: bool, excluded: &[String]) -> serde_json::Value {
    if failover {
        let peers = ["builder-a", "builder-b"]
            .into_iter()
            .filter(|node| !excluded.iter().any(|value| value == node))
            .map(|node| serde_json::json!({"node_id": node, "endpoint": format!("http://{node}.onion")}))
            .collect::<Vec<_>>();
        return serde_json::json!({"type": "PeersSnapshot", "peers": peers});
    }
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

pub(super) fn placed(node_id: &str, status: u16) -> serde_json::Value {
    serde_json::json!({
        "type": "RemotePlaced",
        "task_handle": "daemon-task-retry",
        "peer": {"node_id": node_id, "endpoint": format!("http://{node_id}.onion")},
        "status": status,
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

pub(super) fn result(node_id: &str, failover: bool) -> serde_json::Value {
    let infrastructure_failure = failover && node_id == "builder-a";
    remote_response(GetTaskResultResponse {
        success: !infrastructure_failure,
        exit_code: Some(if infrastructure_failure { 137 } else { 0 }),
        status: if infrastructure_failure {
            "failure"
        } else {
            "success"
        }
        .into(),
        started_at: 0,
        finished_at: 0,
        duration_ms: 0,
        node_id: node_id.into(),
        transport_kind: "tor".into(),
        runtime: None,
        runtime_engine: None,
        outputs: Vec::new(),
        stdout_tail: None,
        stderr_tail: infrastructure_failure.then(|| "builder-a exited 137".into()),
        failure_kind: infrastructure_failure
            .then_some(tak_proto::RemoteFailureKind::ContainerOom as i32),
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
