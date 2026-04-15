use std::time::Duration;

use prost::Message;
use tak_core::model::RemoteTransportKind;
use tak_proto::GetTaskResultResponse;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use super::{StrictRemoteTarget, parse_remote_protocol_result, remote_protocol_http_request};

#[tokio::test]
async fn remote_protocol_http_request_reads_a_complete_http_body_without_waiting_for_eof() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
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

#[test]
fn parse_remote_protocol_result_preserves_failure_stderr_tail() {
    let target = StrictRemoteTarget {
        node_id: "builder-a".into(),
        endpoint: "http://127.0.0.1:65535".into(),
        transport_kind: RemoteTransportKind::Direct,
        bearer_token: "secret".into(),
        runtime: None,
    };
    let response = GetTaskResultResponse {
        success: false,
        exit_code: Some(1),
        status: "failure".into(),
        started_at: 0,
        finished_at: 0,
        duration_ms: 0,
        node_id: "builder-a".into(),
        transport_kind: "direct".into(),
        runtime: None,
        runtime_engine: None,
        outputs: Vec::new(),
        stdout_tail: None,
        stderr_tail: Some("declared output path `out/missing.txt` was not created".into()),
    };

    let parsed = parse_remote_protocol_result(&target, &response.encode_to_vec())
        .expect("result should parse");

    assert!(!parsed.success);
    assert_eq!(
        parsed.failure_detail.as_deref(),
        Some("declared output path `out/missing.txt` was not created")
    );
}
