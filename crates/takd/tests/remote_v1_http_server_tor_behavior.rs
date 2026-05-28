//! Integration contract test for Tor remote V1 HTTP serving entrypoint.
#![allow(clippy::await_holding_lock)]

use crate::support;

use prost::Message;
use tak_proto::{NodeInfo, NodePingResponse};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use support::env::env_lock;
use takd::daemon::remote::{
    RemoteNodeContext, RemoteRuntimeConfig, SubmitAttemptStore, run_remote_v1_http_server,
};

#[tokio::test]
async fn remote_v1_http_server_requires_bearer_auth_for_tor_ping() {
    let _env_lock = env_lock();
    let context = RemoteNodeContext::new(
        NodeInfo {
            node_id: "builder-tor".into(),
            display_name: "builder-tor".into(),
            base_url: "http://builder-tor.onion".into(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "tor".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        },
        "secret".into(),
        RemoteRuntimeConfig::for_tests(),
    );
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("takd.sqlite");
    let store = SubmitAttemptStore::with_db_path(db_path).expect("submit attempt store");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener local addr");
    let server = tokio::spawn(run_remote_v1_http_server(listener, store, context));

    let (missing_head, _) = fetch(
        addr,
        b"GET /v1/node/ping HTTP/1.1\r\nHost: builder-tor.onion\r\nConnection: close\r\n\r\n",
    )
    .await;
    assert!(
        missing_head.starts_with("HTTP/1.1 401 Unauthorized\r\n"),
        "missing auth should be rejected: {missing_head}"
    );

    let (wrong_head, _) = fetch(
        addr,
        b"GET /v1/node/ping HTTP/1.1\r\nHost: builder-tor.onion\r\nAuthorization: Bearer stale\r\nConnection: close\r\n\r\n",
    )
    .await;
    assert!(
        wrong_head.starts_with("HTTP/1.1 401 Unauthorized\r\n"),
        "wrong auth should be rejected: {wrong_head}"
    );

    let (ok_head, body) = fetch(
        addr,
        b"GET /v1/node/ping HTTP/1.1\r\nHost: builder-tor.onion\r\nAuthorization: Bearer secret\r\nConnection: close\r\n\r\n",
    )
    .await;
    assert!(
        ok_head.starts_with("HTTP/1.1 200 OK\r\n"),
        "correct auth should be accepted: {ok_head}"
    );
    let ping = NodePingResponse::decode(body.as_slice()).expect("decode node ping");
    assert_eq!(ping.node_id, "builder-tor");

    server.abort();
}

async fn fetch(addr: std::net::SocketAddr, request: &[u8]) -> (String, Vec<u8>) {
    let mut stream = tokio::net::TcpStream::connect(addr)
        .await
        .expect("connect server");
    stream.write_all(request).await.expect("send request");
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
        .expect("response should contain header terminator");
    let head = String::from_utf8(response[..split].to_vec()).expect("response utf8");
    (head, response[split..].to_vec())
}
