use prost::Message;
use tak_proto::ErrorResponse;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use takd::daemon::remote::{
    RemoteNodeContext, RemoteRuntimeConfig, SubmitAttemptStore, run_remote_v1_http_server,
};

#[tokio::test]
async fn invalid_content_length_returns_explicit_bad_request_reason() {
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("takd.sqlite");
    let store = SubmitAttemptStore::with_db_path(db_path).expect("submit attempt store");
    let context = RemoteNodeContext::new(
        tak_proto::NodeInfo {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
            base_url: "http://127.0.0.1:43123".into(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "direct".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        },
        "secret".into(),
        RemoteRuntimeConfig::for_tests(),
    );
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener local addr");
    let server = tokio::spawn(run_remote_v1_http_server(listener, store, context));

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
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
        .expect("response should contain HTTP header terminator");
    let head = String::from_utf8(response[..split].to_vec()).expect("response utf8");
    let body = &response[split..];
    assert!(
        head.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "expected 400 status for invalid content-length: {head}"
    );
    assert!(
        head.contains("Content-Type: application/x-protobuf\r\n"),
        "expected protobuf error response: {head}"
    );
    let error = ErrorResponse::decode(body).expect("decode error payload");
    assert_eq!(error.message, "invalid_content_length");

    server.abort();
}
