use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Request;
use hyper_util::rt::{TokioExecutor, TokioIo};
use prost::Message;
use tak_proto::{NodeInfo, NodePingResponse};
use takd::{RemoteNodeContext, RemoteRuntimeConfig, SubmitAttemptStore};

#[tokio::test(flavor = "multi_thread")]
async fn remote_v1_http2_server_requires_bearer_auth_for_tor_ping() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let context = RemoteNodeContext::new(
        NodeInfo {
            transport: "tor".into(),
            base_url: "http://builder-h2.onion".into(),
            ..node_info()
        },
        "secret".into(),
        RemoteRuntimeConfig::for_tests(),
    );
    let server = tokio::spawn(takd::run_remote_v1_http_server(listener, store, context));

    let stream = tokio::net::TcpStream::connect(addr).await.expect("connect");
    let (mut sender, connection) =
        hyper::client::conn::http2::handshake(TokioExecutor::new(), TokioIo::new(stream))
            .await
            .expect("h2");
    let connection = tokio::spawn(connection);

    let missing = Request::builder()
        .uri("/v1/node/ping")
        .body(Full::new(Bytes::new()))
        .expect("request");
    let missing = sender.send_request(missing).await.expect("missing auth");
    assert_eq!(missing.status(), 401);

    let wrong = Request::builder()
        .uri("/v1/node/ping")
        .header("Authorization", "Bearer stale")
        .body(Full::new(Bytes::new()))
        .expect("request");
    let wrong = sender.send_request(wrong).await.expect("wrong auth");
    assert_eq!(wrong.status(), 401);

    let ok = Request::builder()
        .uri("/v1/node/ping")
        .header("Authorization", "Bearer secret")
        .body(Full::new(Bytes::new()))
        .expect("request");
    let ok = sender.send_request(ok).await.expect("ok auth");
    assert_eq!(ok.status(), 200);
    let body = ok.into_body().collect().await.expect("body").to_bytes();
    assert_eq!(
        NodePingResponse::decode(body).expect("ping").node_id,
        "builder-h2"
    );

    connection.abort();
    server.abort();
}

fn node_info() -> NodeInfo {
    NodeInfo {
        node_id: "builder-h2".into(),
        display_name: "builder-h2".into(),
        base_url: "http://builder-h2.local".into(),
        healthy: true,
        pools: vec![],
        tags: vec![],
        capabilities: vec![],
        transport: "direct".into(),
        transport_state: "ready".into(),
        transport_detail: String::new(),
    }
}
