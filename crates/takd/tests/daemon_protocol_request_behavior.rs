use takd::{PeersListRequest, Request, Response, StatusRequest};

use crate::support;

use support::protocol::{acquire_request, send_request, send_request_frame};
use support::protocol_server::spawn_protocol_server;

#[path = "daemon_protocol_request_behavior/remote_auth.rs"]
mod remote_auth;
#[path = "daemon_protocol_request_behavior/socket_permissions.rs"]
mod socket_permissions;
#[path = "daemon_protocol_request_behavior/wire_round_trip.rs"]
mod wire_round_trip;

#[tokio::test(flavor = "multi_thread")]
async fn run_server_serves_status_request() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("run/takd.sock");
    let server = spawn_protocol_server(temp.path().join("state/takd.sqlite"), socket_path.clone());

    let status = send_request(
        &socket_path,
        &Request::Status(StatusRequest {
            request_id: "status".into(),
        }),
    )
    .await;

    assert!(matches!(status, Response::StatusSnapshot { .. }));
    server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn run_server_serves_empty_peers_list_request() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("run/takd.sock");
    let server = spawn_protocol_server(temp.path().join("state/takd.sqlite"), socket_path.clone());

    let response = send_request(
        &socket_path,
        &Request::PeersList(PeersListRequest {
            request_id: "peers".into(),
        }),
    )
    .await;

    match response {
        Response::PeersSnapshot { peers, .. } => assert!(peers.is_empty()),
        other => panic!("expected peers snapshot, got {other:?}"),
    }
    server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn run_server_returns_response_frame_for_valid_acquire_lease_request() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("run/takd.sock");
    let server = spawn_protocol_server(temp.path().join("state/takd.sqlite"), socket_path.clone());

    let frame = send_request_frame(
        &socket_path,
        &Request::AcquireLease(acquire_request("acquire")),
    )
    .await;

    assert!(
        !frame.trim().is_empty(),
        "expected response frame for AcquireLease, got EOF"
    );
    let response: Response = serde_json::from_str(frame.trim_end()).expect("decode response");
    assert!(
        matches!(response, Response::LeaseGranted { .. }),
        "expected lease grant, got {response:?}"
    );
    server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn run_server_treats_http_substrings_inside_json_as_protocol_frames() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("run/takd.sock");
    let server = spawn_protocol_server(temp.path().join("state/takd.sqlite"), socket_path.clone());

    let frame = send_request_frame(
        &socket_path,
        &Request::Status(StatusRequest {
            request_id: "status HTTP/1.1".into(),
        }),
    )
    .await;
    let response: Response = serde_json::from_str(frame.trim_end()).expect("decode response");

    assert!(matches!(response, Response::StatusSnapshot { .. }));
    server.abort();
}
