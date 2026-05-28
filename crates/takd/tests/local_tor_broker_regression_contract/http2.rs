use std::sync::atomic::Ordering;

use takd::{TorBroker, new_shared_manager, run_server_with_broker};

use super::support::spawn_http2_remote_that_fails_after_request;
use crate::support as test_support;
use crate::support::local_broker_http::send_raw_http;

#[tokio::test(flavor = "multi_thread")]
async fn local_tor_broker_evicts_failed_http2_post_without_replay() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (remote_addr, remote_requests, connection_count) =
        spawn_http2_remote_that_fails_after_request().await;
    let socket_path = temp.path().join("run/takd.sock");
    let server_socket_path = socket_path.clone();
    let broker = TorBroker::for_test_dial_addr(remote_addr);
    let server = tokio::spawn(async move {
        run_server_with_broker(&server_socket_path, new_shared_manager(), broker).await
    });
    let request = b"POST /v1/tasks/submit HTTP/1.1\r\nHost: builder-h2-post.onion\r\nAuthorization: Bearer secret\r\nX-Tak-Broker-Version: 1\r\nX-Tak-Remote-Node: builder-h2-post\r\nX-Tak-Remote-Endpoint: http://builder-h2-post.onion\r\nX-Tak-Remote-Protocol: h2\r\nX-Tak-Remote-Transport: tor\r\nContent-Length: 4\r\nConnection: close\r\n\r\ntest";

    let response = send_raw_http(&socket_path, request).await;

    assert!(response.starts_with(b"HTTP/1.1 502 Bad Gateway\r\n"));
    assert_eq!(
        *remote_requests.lock().expect("remote request lock"),
        vec!["POST /v1/tasks/submit".to_string()]
    );
    assert_eq!(connection_count.load(Ordering::SeqCst), 1);

    let retry = send_raw_http(&socket_path, request).await;

    assert!(retry.starts_with(b"HTTP/1.1 502 Bad Gateway\r\n"));
    assert_eq!(connection_count.load(Ordering::SeqCst), 2);
    server.abort();
}

#[tokio::test(flavor = "multi_thread")]
async fn local_tor_broker_rejects_oversized_http2_response_before_collecting_body() {
    let temp = tempfile::tempdir().expect("tempdir");
    let remote = test_support::http2_remote::Http2Remote::spawn_with_content_length(
        b"ok".to_vec(),
        512 * 1024 * 1024 + 1,
    )
    .await;
    let socket_path = temp.path().join("run/takd.sock");
    let server_socket_path = socket_path.clone();
    let broker = TorBroker::for_test_dial_addr(remote.addr.clone());
    let server = tokio::spawn(async move {
        run_server_with_broker(&server_socket_path, new_shared_manager(), broker).await
    });
    let request = b"GET /v1/node/info HTTP/1.1\r\nHost: builder-h2-large.onion\r\nAuthorization: Bearer secret\r\nX-Tak-Broker-Version: 1\r\nX-Tak-Remote-Node: builder-h2-large\r\nX-Tak-Remote-Endpoint: http://builder-h2-large.onion\r\nX-Tak-Remote-Protocol: h2\r\nX-Tak-Remote-Transport: tor\r\nConnection: close\r\n\r\n";

    let response = send_raw_http(&socket_path, request).await;

    assert!(response.starts_with(b"HTTP/1.1 502 Bad Gateway\r\n"));
    assert!(String::from_utf8_lossy(&response).contains("response_body_too_large"));
    server.abort();
}
