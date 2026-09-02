use tak_proto::local_daemon::v2::{Operation, Request, Response};

use crate::support;

use support::protocol::{send_raw_frame, send_request, send_request_frame};
use support::protocol_server::spawn_protocol_server;

#[path = "daemon_protocol_request_behavior/socket_permissions.rs"]
mod socket_permissions;

#[tokio::test(flavor = "multi_thread")]
async fn run_server_serves_status_request() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("run/takd.sock");
    let server = spawn_protocol_server(temp.path().join("state/takd.sqlite"), socket_path.clone());

    let status = send_request(
        &socket_path,
        &Request {
            request_id: "status".into(),
            operation: Operation::GetDaemonStatus {},
        },
    )
    .await;

    assert!(matches!(status, Response::DaemonStatus { .. }));
    server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn run_server_serves_empty_protocol_v2_remote_status_request() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("run/takd.sock");
    let server = spawn_protocol_server(temp.path().join("state/takd.sqlite"), socket_path.clone());

    let response = send_request(
        &socket_path,
        &Request {
            request_id: "peers".into(),
            operation: Operation::GetRemoteStatus {
                node_ids: Vec::new(),
            },
        },
    )
    .await;

    match response {
        Response::RemoteStatus { remotes, .. } => assert!(remotes.is_empty()),
        other => panic!("expected remote status, got {other:?}"),
    }
    server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn run_server_rejects_versionless_acquire_lease_request_with_upgrade_guidance() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("run/takd.sock");
    let server = spawn_protocol_server(temp.path().join("state/takd.sqlite"), socket_path.clone());

    let frame = send_raw_frame(
        &socket_path,
        r#"{"type":"AcquireLease","request_id":"acquire","client":{"user":"alice","pid":7,"session_id":"s"},"task":{"label":"//:check","attempt":1},"needs":[],"ttl_ms":30000}"#,
    )
    .await;

    assert!(
        !frame.trim().is_empty(),
        "expected response frame for AcquireLease, got EOF"
    );
    assert!(frame.contains(r#""code":"protocol_version_unsupported""#));
    assert!(frame.contains("Upgrade tak, takd, and workers together"));
    server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn run_server_treats_http_substrings_inside_json_as_protocol_frames() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("run/takd.sock");
    let server = spawn_protocol_server(temp.path().join("state/takd.sqlite"), socket_path.clone());

    let frame = send_request_frame(
        &socket_path,
        &Request {
            request_id: "status HTTP/1.1".into(),
            operation: Operation::GetDaemonStatus {},
        },
    )
    .await;
    let response = tak_proto::local_daemon::v2::decode_response(
        frame.trim_end().as_bytes(),
        "status HTTP/1.1",
    )
    .expect("decode response");

    assert!(matches!(response, Response::DaemonStatus { .. }));
    server.abort();
}
