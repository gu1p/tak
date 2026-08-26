use std::time::Duration;

use prost::Message;
use tak_core::model::{BackoffDef, TaskLabel};
use tak_proto::{ErrorResponse, GetTaskResultResponse, PollTaskEventsResponse, RemoteEvent};
use tokio::net::TcpListener;

use crate::engine::remote_models::{StrictRemoteTarget, StrictRemoteTransportKind};
use crate::engine::remote_result_fetch::ResultFetchPolicy;

mod http;
mod observer;

pub(super) use http::spawn_http_server;
pub(super) use observer::CapturingObserver;

pub(super) fn direct_target(endpoint: String) -> StrictRemoteTarget {
    StrictRemoteTarget {
        node_id: "builder-a".into(),
        endpoint,
        transport_kind: StrictRemoteTransportKind::Direct,
        bearer_token: "secret".into(),
        runtime: None,
        remote_selection: tak_core::model::RemoteSelectionSpec::Sequential,
        required_pool: None,
        required_tags: Vec::new(),
        required_capabilities: Vec::new(),
        daemon_task_handle: None,
        excluded_node_ids: Vec::new(),
    }
}

pub(super) fn task_label() -> TaskLabel {
    TaskLabel {
        package: "//".into(),
        name: "demo".into(),
    }
}

pub(super) fn fast_policy() -> ResultFetchPolicy {
    ResultFetchPolicy {
        max_attempts: 3,
        not_found_grace: 3,
        backoff: BackoffDef::Fixed { seconds: 0.0 },
        not_found_backoff: Duration::ZERO,
    }
}

pub(super) fn result_body(success: bool) -> Vec<u8> {
    GetTaskResultResponse {
        success,
        status: if success { "success" } else { "failure" }.into(),
        node_id: "builder-a".into(),
        transport_kind: "direct".into(),
        ..GetTaskResultResponse::default()
    }
    .encode_to_vec()
}

pub(super) fn error_body(message: &str) -> Vec<u8> {
    ErrorResponse {
        message: message.to_string(),
    }
    .encode_to_vec()
}

pub(super) fn events_body(events: Vec<RemoteEvent>, done: bool) -> Vec<u8> {
    PollTaskEventsResponse { events, done }.encode_to_vec()
}

pub(super) fn stdout_event(seq: u64, bytes: &[u8]) -> RemoteEvent {
    RemoteEvent {
        seq,
        kind: "TASK_STDOUT_CHUNK".into(),
        chunk_bytes: bytes.to_vec(),
        ..RemoteEvent::default()
    }
}

pub(super) async fn bind_local() -> (TcpListener, String) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    (listener, format!("http://{addr}"))
}
