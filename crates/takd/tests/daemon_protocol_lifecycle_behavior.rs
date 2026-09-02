use crate::support;

use support::protocol::send_raw_frame;
use support::protocol_server::spawn_protocol_server;

#[tokio::test(flavor = "multi_thread")]
async fn run_server_rejects_versionless_renew_lease_request() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("run/takd.sock");
    let server = spawn_protocol_server(temp.path().join("state/takd.sqlite"), socket_path.clone());

    let renewed = send_raw_frame(
        &socket_path,
        r#"{"type":"RenewLease","request_id":"renew","lease_id":"lease-1","ttl_ms":15000}"#,
    )
    .await;

    assert!(renewed.contains(r#""code":"protocol_version_unsupported""#));
    server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn run_server_rejects_versionless_release_lease_request() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("run/takd.sock");
    let server = spawn_protocol_server(temp.path().join("state/takd.sqlite"), socket_path.clone());

    let released = send_raw_frame(
        &socket_path,
        r#"{"type":"ReleaseLease","request_id":"release","lease_id":"lease-1"}"#,
    )
    .await;

    assert!(released.contains(r#""code":"protocol_version_unsupported""#));
    server.abort();
}
