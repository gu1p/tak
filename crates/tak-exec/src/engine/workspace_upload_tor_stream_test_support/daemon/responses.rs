use std::sync::Arc;

use prost::Message;
use tak_proto::{AppendWorkspaceUploadResponse, BeginWorkspaceUploadResponse};
use tokio::sync::Mutex;

use super::super::http::{json_string_field, peers_response, protobuf_http_response};
use super::UploadState;

pub(super) async fn daemon_response(
    request: &str,
    state: Arc<Mutex<UploadState>>,
) -> serde_json::Value {
    if request.contains(r#""type":"PeersEligible""#) {
        return peers_response();
    }
    if request.contains(r#""type":"ForwardRemoteHttp""#) {
        return status_response(request, state).await;
    }
    serde_json::json!({"type": "Error", "message": "unexpected request", "retryable": false})
}

pub(super) async fn stream_response(
    offset: u64,
    body: Vec<u8>,
    state: Arc<Mutex<UploadState>>,
) -> Option<Vec<u8>> {
    let mut state = state.lock().await;
    state.stream_offsets.push(offset);
    if offset != state.bytes.len() as u64 {
        return Some(protobuf_http_response(AppendWorkspaceUploadResponse {
            offset: state.bytes.len() as u64,
            complete: false,
        }));
    }
    if state.always_drop_without_progress {
        return None;
    }
    if let Some(commit) = state.dropped_commits.pop_front() {
        let commit = commit.min(body.len());
        state.bytes.extend_from_slice(&body[..commit]);
        return None;
    }
    state.bytes.extend_from_slice(&body);
    Some(protobuf_http_response(AppendWorkspaceUploadResponse {
        offset: state.bytes.len() as u64,
        complete: state.bytes.len() as u64 == state.expected_size,
    }))
}

async fn status_response(request: &str, state: Arc<Mutex<UploadState>>) -> serde_json::Value {
    let node_id = json_string_field(request, "node_id").unwrap_or_default();
    let path = json_string_field(request, "path").unwrap_or_default();
    let mut state = state.lock().await;
    if path.contains("/wormhole") {
        state.wormhole_attempts += 1;
        if state.retryable_wormhole_error {
            return serde_json::json!({
                "type": "Error",
                "message": "temporary wormhole preflight failure",
                "retryable": true,
            });
        }
        if state.unsupported_wormhole {
            return serde_json::json!({
                "type": "RemoteHttpResponse",
                "status": 404,
                "headers": [],
                "body": [],
            });
        }
    }
    state.status_nodes.push(node_id);
    serde_json::json!({
        "type": "RemoteHttpResponse",
        "status": 200,
        "headers": [],
        "body": BeginWorkspaceUploadResponse {
            upload_id: "upload".to_string(),
            offset: state.bytes.len() as u64,
            complete: state.bytes.len() as u64 == state.expected_size,
        }.encode_to_vec(),
    })
}
