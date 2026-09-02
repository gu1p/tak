use tak_proto::local_daemon::v2::{Operation, Request, Response};

use crate::support;
use support::protocol::send_request;
use support::protocol_server::spawn_protocol_server;

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn run_server_binds_owner_only_socket() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};

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
    let metadata = std::fs::metadata(&socket_path).expect("socket metadata");
    let socket_mode = metadata.permissions().mode() & 0o777;
    // SAFETY: geteuid has no preconditions and only reads process credentials.
    let expected_uid = unsafe { libc::geteuid() };
    assert_eq!(metadata.uid(), expected_uid);
    assert_ne!(socket_mode & 0o200, 0, "owner cannot connect to socket");
    assert_eq!(socket_mode & 0o022, 0, "non-owner can connect to socket");
    server.abort();
}
