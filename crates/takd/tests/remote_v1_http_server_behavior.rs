//! Integration contract test for remote V1 HTTP serving entrypoint.

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use takd::daemon::remote::{SubmitAttemptStore, run_remote_v1_http_server};

#[tokio::test]
async fn remote_v1_http_server_serves_capabilities_endpoint() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("takd.sqlite");
    let store = SubmitAttemptStore::with_db_path(db_path).expect("submit attempt store");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener local addr");
    let server = tokio::spawn(run_remote_v1_http_server(listener, store));

    let mut stream = tokio::net::TcpStream::connect(addr)
        .await
        .expect("connect server");
    stream
        .write_all(
            b"GET /v1/node/capabilities HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        )
        .await
        .expect("send capabilities request");
    stream.shutdown().await.expect("shutdown write side");

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .expect("read response");
    let response = String::from_utf8(response).expect("response utf8");
    assert!(
        response.starts_with("HTTP/1.1 200 OK\r\n"),
        "capabilities endpoint should return 200: {response}"
    );
    assert!(
        response.contains("\"compatible\":true"),
        "capabilities response should include compatibility marker: {response}"
    );

    server.abort();
}
