use prost::Message;
use tak_core::model::Scope;
use tak_proto::NodeInfo;

use crate::support;
use support::local_broker_http::{response_body, send_broker_get, send_raw_http};
use support::recording_remote::RecordingRemote;

#[tokio::test(flavor = "multi_thread")]
async fn local_tor_broker_forwards_remote_v1_http_without_broker_headers() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote = RecordingRemote::spawn("builder-broker").await;
    let socket_path = temp.path().join("run/takd.sock");
    let server_socket_path = socket_path.clone();
    let broker = crate::support::local_runtime::tor_broker(remote.addr.clone());
    let server = tokio::spawn(async move {
        let manager = crate::support::local_runtime::in_memory_lease_manager();
        manager
            .lock()
            .expect("manager lock")
            .set_capacity("cpu", Scope::Machine, None, 8.0);
        crate::support::local_runtime::run_local_server(&server_socket_path, manager, broker).await
    });

    let response = send_broker_get(&socket_path, "builder-broker").await;
    assert!(response.starts_with(b"HTTP/1.1 200 OK\r\n"));
    let node = NodeInfo::decode(response_body(&response)).expect("decode node info");

    assert_eq!(node.node_id, "builder-broker");
    assert!(
        remote
            .single_request()
            .contains("Authorization: Bearer secret")
    );
    assert!(!remote.single_request().contains("X-Tak-Broker-"));
    server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn local_tor_broker_rejects_requests_without_broker_version() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote = RecordingRemote::spawn("builder-invalid").await;
    let socket_path = temp.path().join("run/takd.sock");
    let server_socket_path = socket_path.clone();
    let broker = crate::support::local_runtime::tor_broker(remote.addr.clone());
    let server = crate::support::local_runtime::spawn_local_server(server_socket_path, broker);
    let request = b"GET /v1/node/info HTTP/1.1\r\nHost: builder-invalid.onion\r\nX-Tak-Remote-Node: builder-invalid\r\nX-Tak-Remote-Endpoint: http://builder-invalid.onion\r\nX-Tak-Remote-Transport: tor\r\nConnection: close\r\n\r\n";

    let response = send_raw_http(&socket_path, request).await;

    assert!(response.starts_with(b"HTTP/1.1 400 Bad Request\r\n"));
    assert!(String::from_utf8_lossy(&response).contains("missing_broker_version"));
    server.abort();
}
