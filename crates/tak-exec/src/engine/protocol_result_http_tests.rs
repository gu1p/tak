use std::time::Duration;

use tak_core::model::RemoteTransportKind;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::{StrictRemoteTarget, remote_protocol_http_request};

#[tokio::test]
async fn remote_protocol_http_request_reads_a_complete_http_body_without_waiting_for_eof() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept request");
        let mut request = [0_u8; 256];
        let _ = stream.read(&mut request).await.expect("read request");
        let body = b"hello";
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(head.as_bytes()).await.expect("write head");
        stream.write_all(body).await.expect("write body");
        stream.flush().await.expect("flush body");
        tokio::time::sleep(Duration::from_millis(200)).await;
    });
    let target = StrictRemoteTarget {
        node_id: "builder-a".into(),
        endpoint: format!("http://{addr}"),
        transport_kind: RemoteTransportKind::Direct,
        bearer_token: "secret".into(),
        runtime: None,
    };

    let (_, body) = remote_protocol_http_request(
        &target,
        "GET",
        "/v1/tasks/example/result",
        None,
        "result",
        Duration::from_millis(50),
    )
    .await
    .expect("response should complete from Content-Length alone");
    assert_eq!(body, b"hello");
    server.await.expect("server task");
}
