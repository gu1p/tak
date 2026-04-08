#![allow(dead_code)]

use prost::Message;
use tak_proto::{NodeInfo, NodeStatusResponse};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{Duration, sleep};

pub async fn fetch_node_info(socket_addr: &str, host_header: &str, bearer_token: &str) -> NodeInfo {
    let mut stream = tokio::net::TcpStream::connect(socket_addr)
        .await
        .expect("connect remote v1 server");
    let request = format!(
        "GET /v1/node/info HTTP/1.1\r\nHost: {host_header}\r\nAuthorization: Bearer {bearer_token}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .await
        .expect("write node info request");
    stream.shutdown().await.expect("shutdown write side");

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .expect("read node info response");
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
        .expect("response should contain header terminator");
    NodeInfo::decode(&response[split..]).expect("decode node info")
}

pub async fn wait_for_node_info(
    socket_addr: &str,
    host_header: &str,
    bearer_token: &str,
) -> NodeInfo {
    for _ in 0..50 {
        if tokio::net::TcpStream::connect(socket_addr).await.is_ok() {
            return fetch_node_info(socket_addr, host_header, bearer_token).await;
        }
        sleep(Duration::from_millis(20)).await;
    }
    panic!("timed out waiting for node info at {socket_addr}");
}

pub async fn fetch_node_status(
    socket_addr: &str,
    host_header: &str,
    bearer_token: &str,
) -> NodeStatusResponse {
    let mut stream = tokio::net::TcpStream::connect(socket_addr)
        .await
        .expect("connect remote v1 server");
    let request = format!(
        "GET /v1/node/status HTTP/1.1\r\nHost: {host_header}\r\nAuthorization: Bearer {bearer_token}\r\nConnection: close\r\n\r\n"
    );
    stream
        .write_all(request.as_bytes())
        .await
        .expect("write node status request");
    stream.shutdown().await.expect("shutdown write side");

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .expect("read node status response");
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
        .expect("response should contain header terminator");
    NodeStatusResponse::decode(&response[split..]).expect("decode node status")
}
