use takd::{Request, Response, StatusRequest};

use crate::support;

use support::protocol::send_request;
use support::protocol_server::spawn_protocol_server;

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn run_server_preserves_existing_socket_parent_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().expect("tempdir");
    let socket_parent = temp.path().join("shared");
    std::fs::create_dir(&socket_parent).expect("create shared socket parent");
    std::fs::set_permissions(&socket_parent, std::fs::Permissions::from_mode(0o755))
        .expect("set shared parent mode");
    let socket_path = socket_parent.join("takd.sock");
    let server = spawn_protocol_server(temp.path().join("state/takd.sqlite"), socket_path.clone());

    let status = send_request(
        &socket_path,
        &Request::Status(StatusRequest {
            request_id: "status".into(),
        }),
    )
    .await;
    assert!(matches!(status, Response::StatusSnapshot { .. }));

    let parent_mode = std::fs::metadata(&socket_parent)
        .expect("parent metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(parent_mode, 0o755);
    server.abort();
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn run_server_locks_down_created_socket_parent() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::tempdir().expect("tempdir");
    let socket_parent = temp.path().join("run");
    let socket_path = socket_parent.join("takd.sock");
    let server = spawn_protocol_server(temp.path().join("state/takd.sqlite"), socket_path.clone());

    let status = send_request(
        &socket_path,
        &Request::Status(StatusRequest {
            request_id: "status".into(),
        }),
    )
    .await;
    assert!(matches!(status, Response::StatusSnapshot { .. }));

    let parent_mode = std::fs::metadata(&socket_parent)
        .expect("parent metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(parent_mode, 0o700);
    server.abort();
}
