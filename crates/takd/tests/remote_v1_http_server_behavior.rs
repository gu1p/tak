//! Integration contract test for remote V1 HTTP serving entrypoint.

use prost::Message;
use tak_proto::{NodeInfo, NodePingResponse};

#[path = "remote_v1_http_server_behavior/support.rs"]
mod support;

#[tokio::test]
async fn remote_v1_http_server_serves_protobuf_node_info() {
    let (head, body) = support::fetch("/v1/node/info").await;

    assert!(
        head.starts_with("HTTP/1.1 200 OK\r\n"),
        "node info endpoint should return 200: {head}"
    );
    assert!(
        head.contains("Content-Type: application/x-protobuf\r\n"),
        "node info endpoint should return protobuf: {head}"
    );
    let node = NodeInfo::decode(body.as_slice()).expect("decode node info");
    assert_eq!(node.node_id, "builder-a");
}

#[tokio::test]
async fn remote_v1_http_server_serves_protobuf_node_ping() {
    let (head, body) = support::fetch("/v1/node/ping").await;

    assert!(
        head.starts_with("HTTP/1.1 200 OK\r\n"),
        "node ping endpoint should return 200: {head}"
    );
    assert!(
        head.contains("Content-Type: application/x-protobuf\r\n"),
        "node ping endpoint should return protobuf: {head}"
    );
    let ping = NodePingResponse::decode(body.as_slice()).expect("decode node ping");
    assert_eq!(ping.node_id, "builder-a");
    assert_eq!(ping.protocol_version, "v1");
    assert_eq!(ping.health, "healthy");
}
