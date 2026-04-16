//! Integration contract test for Tor remote V1 HTTP serving entrypoint.

use prost::Message;
use tak_proto::NodeInfo;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use takd::daemon::remote::{RemoteNodeContext, SubmitAttemptStore, run_remote_v1_http_server};

#[tokio::test]
async fn remote_v1_http_server_allows_tor_requests_without_authorization_header() {
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
    );
    let temp = tempfile::tempdir().expect("tempdir");
    let db_path = temp.path().join("takd.sqlite");
    let store = SubmitAttemptStore::with_db_path(db_path).expect("submit attempt store");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener local addr");
    let server = tokio::spawn(run_remote_v1_http_server(listener, store, context));

    let mut stream = tokio::net::TcpStream::connect(addr)
        .await
        .expect("connect server");
    stream
        .write_all(
            b"GET /v1/node/info HTTP/1.1\r\nHost: builder-tor.onion\r\nConnection: close\r\n\r\n",
        )
        .await
        .expect("send node info request");
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
    let body = &response[split..];
    assert!(
        head.starts_with("HTTP/1.1 200 OK\r\n"),
        "node info endpoint should return 200: {head}"
    );
    let node = NodeInfo::decode(body).expect("decode node info");
    assert_eq!(node.node_id, "builder-tor");

    server.abort();
}
