use takd::{TorBroker, new_shared_manager, run_server_with_broker};

use crate::support::local_broker_http::send_raw_http;

#[tokio::test(flavor = "multi_thread")]
async fn local_tor_broker_rejects_oversized_request_body_before_allocation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let socket_path = temp.path().join("run/takd.sock");
    let server_socket_path = socket_path.clone();
    let broker = TorBroker::for_test_dial_addr("127.0.0.1:9".into());
    let server = tokio::spawn(async move {
        run_server_with_broker(&server_socket_path, new_shared_manager(), broker).await
    });
    let request = b"POST /v1/tasks/submit HTTP/1.1\r\nHost: builder-too-large.onion\r\nX-Tak-Broker-Version: 1\r\nX-Tak-Remote-Node: builder-too-large\r\nX-Tak-Remote-Endpoint: http://builder-too-large.onion\r\nX-Tak-Remote-Transport: tor\r\nContent-Length: 536870913\r\nConnection: close\r\n\r\n";

    let response = send_raw_http(&socket_path, request).await;

    assert!(response.starts_with(b"HTTP/1.1 400 Bad Request\r\n"));
    assert!(String::from_utf8_lossy(&response).contains("body_too_large"));
    server.abort();
}
