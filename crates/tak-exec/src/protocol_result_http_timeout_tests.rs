#![cfg(test)]

use std::time::Duration;

use prost::Message;
use tak_proto::GetTaskResultResponse;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::engine::protocol_result_http::{
    remote_protocol_http_request, try_remote_protocol_result,
};
use crate::engine::remote_models::{StrictRemoteTarget, StrictRemoteTransportKind};

#[tokio::test]
async fn remote_protocol_http_request_timeout_mentions_endpoint_and_transport() {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener addr");
    let server = tokio::spawn(async move {
        let (_stream, _) = listener.accept().await.expect("accept request");
        tokio::time::sleep(Duration::from_millis(200)).await;
    });
    let endpoint = format!("http://{addr}");
    let target = StrictRemoteTarget {
        node_id: "builder-a".into(),
        endpoint: endpoint.clone(),
        transport_kind: StrictRemoteTransportKind::Direct,
        bearer_token: "secret".into(),
        runtime: None,
    };

    let err = remote_protocol_http_request(
        &target,
        "GET",
        "/v1/node/info",
        None,
        "node info",
        Duration::from_millis(50),
    )
    .await
    .expect_err("request should time out");
    let rendered = format!("{err:#}");

    assert!(rendered.contains(&format!(
        "infra error: remote node builder-a at {endpoint} via direct node info request timed out"
    )));
    server.await.expect("server task");
}

#[tokio::test]
async fn remote_protocol_result_allows_busy_direct_daemons_more_than_one_second() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("listener addr");
    let server = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept request");
        let mut request = [0_u8; 512];
        let _ = stream.read(&mut request).await.expect("read request");
        tokio::time::sleep(Duration::from_millis(1200)).await;
        let body = GetTaskResultResponse {
            success: true,
            status: "success".into(),
            node_id: "builder-a".into(),
            transport_kind: "direct".into(),
            ..GetTaskResultResponse::default()
        }
        .encode_to_vec();
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/x-protobuf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream.write_all(head.as_bytes()).await.expect("write head");
        stream.write_all(&body).await.expect("write body");
    });
    let target = StrictRemoteTarget {
        node_id: "builder-a".into(),
        endpoint: format!("http://{addr}"),
        transport_kind: StrictRemoteTransportKind::Direct,
        bearer_token: "secret".into(),
        runtime: None,
    };

    let result = try_remote_protocol_result(&target, "task-run", 1)
        .await
        .expect("result request should wait for busy daemon")
        .expect("result");

    assert!(result.success);
    server.await.expect("server task");
}
