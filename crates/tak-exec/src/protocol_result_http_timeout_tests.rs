#![cfg(test)]

use std::time::Duration;

use tokio::net::TcpListener;

use crate::engine::protocol_result_http::remote_protocol_http_request;
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
