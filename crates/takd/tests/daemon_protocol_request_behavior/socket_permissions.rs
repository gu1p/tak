use takd::{Request, Response, StatusRequest};

use crate::support;
use support::protocol::send_request;
use support::protocol_server::spawn_protocol_server;

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn run_server_binds_owner_only_socket() {
    use std::os::unix::fs::PermissionsExt;

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
    let socket_mode = std::fs::metadata(&socket_path)
        .expect("socket metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(socket_mode, 0o600);
    server.abort();
}
