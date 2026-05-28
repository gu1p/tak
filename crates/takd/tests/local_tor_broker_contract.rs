use prost::Message;
use tak_core::model::Scope;
use tak_proto::NodeInfo;
use takd::{TorBroker, new_shared_manager, run_server_with_broker};

use crate::support;
use support::local_broker_http::{response_body, send_broker_get, send_raw_http};
use support::recording_remote::RecordingRemote;

#[tokio::test(flavor = "multi_thread")]
async fn local_tor_broker_forwards_remote_v1_http_without_broker_headers() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote = RecordingRemote::spawn("builder-broker").await;
    let socket_path = temp.path().join("run/takd.sock");
    let server_socket_path = socket_path.clone();
    let broker = TorBroker::for_test_dial_addr(remote.addr.clone());
    let server = tokio::spawn(async move {
        let manager = new_shared_manager();
        manager
            .lock()
            .expect("manager lock")
            .set_capacity("cpu", Scope::Machine, None, 8.0);
        run_server_with_broker(&server_socket_path, manager, broker).await
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
async fn local_tor_broker_reuses_bootstrapped_transport_state_across_requests() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote = RecordingRemote::spawn("builder-reuse").await;
    let socket_path = temp.path().join("run/takd.sock");
    let server_socket_path = socket_path.clone();
    let broker = TorBroker::for_test_dial_addr(remote.addr.clone());
    let broker_for_assertion = broker.clone();
    let server = tokio::spawn(async move {
        run_server_with_broker(&server_socket_path, new_shared_manager(), broker).await
    });

    let _ = send_broker_get(&socket_path, "builder-reuse").await;
    let _ = send_broker_get(&socket_path, "builder-reuse").await;

    assert_eq!(broker_for_assertion.bootstrap_count(), 1);
    assert_eq!(remote.request_count(), 2);
    server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn local_tor_broker_starts_bootstrap_when_socket_is_ready() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote = RecordingRemote::spawn("builder-warm").await;
    let socket_path = temp.path().join("run/takd.sock");
    let server_socket_path = socket_path.clone();
    let broker = TorBroker::for_test_dial_addr(remote.addr.clone());
    let broker_for_assertion = broker.clone();
    let server = tokio::spawn(async move {
        run_server_with_broker(&server_socket_path, new_shared_manager(), broker).await
    });

    for _ in 0..50 {
        if broker_for_assertion.bootstrap_count() == 1 {
            server.abort();
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    server.abort();
    panic!("broker did not start bootstrap before first request");
}

#[tokio::test(flavor = "multi_thread")]
async fn local_tor_broker_rejects_requests_without_broker_version() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote = RecordingRemote::spawn("builder-invalid").await;
    let socket_path = temp.path().join("run/takd.sock");
    let server_socket_path = socket_path.clone();
    let broker = TorBroker::for_test_dial_addr(remote.addr.clone());
    let server = tokio::spawn(async move {
        run_server_with_broker(&server_socket_path, new_shared_manager(), broker).await
    });
    let request = b"GET /v1/node/info HTTP/1.1\r\nHost: builder-invalid.onion\r\nX-Tak-Remote-Node: builder-invalid\r\nX-Tak-Remote-Endpoint: http://builder-invalid.onion\r\nX-Tak-Remote-Transport: tor\r\nConnection: close\r\n\r\n";

    let response = send_raw_http(&socket_path, request).await;

    assert!(response.starts_with(b"HTTP/1.1 400 Bad Request\r\n"));
    assert!(String::from_utf8_lossy(&response).contains("missing_broker_version"));
    server.abort();
}
