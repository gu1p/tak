use prost::Message;
use tak_core::model::{ContainerRuntimeSourceSpec, RemoteRuntimeSpec};
use tak_proto::{SubmitTaskRequest, SubmitTaskResponse};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixListener;

use super::super::super::remote_models::{StrictRemoteTarget, StrictRemoteTransportKind};

pub(super) fn submit_request_body() -> Vec<u8> {
    SubmitTaskRequest {
        task_run_id: "task-1".into(),
        ..SubmitTaskRequest::default()
    }
    .encode_to_vec()
}

pub(super) async fn spawn_submit_daemon(
    socket_path: &std::path::Path,
) -> tokio::task::JoinHandle<String> {
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent).expect("socket parent");
    }
    let listener = UnixListener::bind(socket_path).expect("bind fake daemon");
    tokio::spawn(async move {
        let (stream, _) = listener.accept().await.expect("accept daemon request");
        let (reader_half, mut writer_half) = stream.into_split();
        let mut request = String::new();
        BufReader::new(reader_half)
            .read_line(&mut request)
            .await
            .expect("read request");
        writer_half
            .write_all(submit_response().to_string().as_bytes())
            .await
            .expect("write response");
        writer_half.write_all(b"\n").await.expect("write newline");
        request
    })
}

fn submit_response() -> serde_json::Value {
    serde_json::json!({
        "type": "RemotePlaced",
        "request_id": "place",
        "task_handle": "daemon-task-7",
        "peer": placed_peer(),
        "status": 200,
        "headers": [],
        "body": submit_response_body(),
    })
}

fn placed_peer() -> serde_json::Value {
    serde_json::json!({
        "node_id": "builder-daemon-choice",
        "display_name": "Builder Daemon Choice",
        "transport": "tor",
        "endpoint": "http://builder-daemon-choice.onion",
        "state": "connected",
        "last_heartbeat_ms": 1,
        "last_successful_connection_ms": 1,
        "last_error_summary": null,
        "active_job_count": 0,
        "queue_depth": 0,
        "resource_summary": "cpu_available=8.00 memory_available_mb=16384",
        "protocol_version": "v1",
        "heartbeat_rtt_ms": 5,
        "reconnect_attempts": 0,
        "pools": ["build"],
        "tags": ["linux"],
        "capabilities": ["docker"]
    })
}

fn submit_response_body() -> Vec<u8> {
    SubmitTaskResponse {
        accepted: true,
        attached: false,
        idempotency_key: "task-1:1".into(),
        remote_worker: true,
    }
    .encode_to_vec()
}

pub(super) fn tor_target() -> StrictRemoteTarget {
    StrictRemoteTarget {
        node_id: "builder-client-choice".into(),
        endpoint: "http://builder-client-choice.onion".into(),
        transport_kind: StrictRemoteTransportKind::Tor,
        bearer_token: "secret".into(),
        runtime: Some(runtime()),
        remote_selection: tak_core::model::RemoteSelectionSpec::Shuffle,
        required_pool: Some("build".into()),
        required_tags: vec!["linux".into()],
        required_capabilities: vec!["docker".into()],
        daemon_task_handle: None,
    }
}

fn runtime() -> RemoteRuntimeSpec {
    RemoteRuntimeSpec::Containerized {
        source: ContainerRuntimeSourceSpec::Image {
            image: "alpine:3.20".into(),
        },
        resource_limits: None,
    }
}
