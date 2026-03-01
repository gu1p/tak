use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use takd::daemon::remote::{SubmitAttemptStore, run_remote_v1_http_server};

#[tokio::test]
async fn invalid_content_length_returns_explicit_bad_request_reason() {
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
    let request = concat!(
        "POST /v1/tasks/submit HTTP/1.1\r\n",
        "Host: 127.0.0.1\r\n",
        "Content-Type: application/json\r\n",
        "Content-Length: not-a-number\r\n",
        "Connection: close\r\n",
        "\r\n",
        "{\"task_run_id\":\"r-1\",\"attempt\":1,\"selected_node_id\":\"node-1\"}"
    );
    stream
        .write_all(request.as_bytes())
        .await
        .expect("write request");
    stream.shutdown().await.expect("shutdown write side");

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .expect("read response");
    let response = String::from_utf8(response).expect("response utf8");
    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "expected 400 status for invalid content-length: {response}"
    );
    assert!(
        response.contains(r#""reason":"invalid_content_length""#),
        "expected explicit invalid_content_length reason: {response}"
    );

    server.abort();
}
