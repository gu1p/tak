use std::time::Duration;

use takd::{TorBroker, new_shared_manager, run_server_with_broker};

use super::support::{
    spawn_remote_that_keeps_connection_open, spawn_remote_with_oversized_content_length,
};
use crate::support::local_broker_http::send_broker_get;

#[tokio::test(flavor = "multi_thread")]
async fn local_tor_broker_completes_content_length_response_without_remote_eof() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote_addr = spawn_remote_that_keeps_connection_open().await;
    let socket_path = temp.path().join("run/takd.sock");
    let server_socket_path = socket_path.clone();
    let broker = TorBroker::for_test_dial_addr(remote_addr);
    let server = tokio::spawn(async move {
        run_server_with_broker(&server_socket_path, new_shared_manager(), broker).await
    });

    let response = tokio::time::timeout(
        Duration::from_millis(50),
        send_broker_get(&socket_path, "builder-length"),
    )
    .await
    .expect("broker should finish from Content-Length without remote EOF");

    assert!(response.starts_with(b"HTTP/1.1 200 OK\r\n"));
    assert!(response.ends_with(b"ok"));
    server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn local_tor_broker_rejects_oversized_http1_response_before_allocation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote_addr = spawn_remote_with_oversized_content_length().await;
    let socket_path = temp.path().join("run/takd.sock");
    let server_socket_path = socket_path.clone();
    let broker = TorBroker::for_test_dial_addr(remote_addr);
    let server = tokio::spawn(async move {
        run_server_with_broker(&server_socket_path, new_shared_manager(), broker).await
    });

    let response = send_broker_get(&socket_path, "builder-large-response").await;

    assert!(response.starts_with(b"HTTP/1.1 502 Bad Gateway\r\n"));
    assert!(String::from_utf8_lossy(&response).contains("response_body_too_large"));
    server.abort();
}
