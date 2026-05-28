use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::Request;
use hyper_util::rt::{TokioExecutor, TokioIo};
use prost::Message;
use tak_proto::NodeInfo;
use takd::{RemoteNodeContext, RemoteRuntimeConfig, SubmitAttemptStore};

#[path = "remote_v1_http2_server_contract/auth.rs"]
mod auth;

#[tokio::test(flavor = "multi_thread")]
async fn remote_v1_server_accepts_http2_requests_on_one_connection() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("agent.sqlite")).expect("store");
    let context = RemoteNodeContext::new(
        node_info(),
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

    assert_node(&mut sender).await;
    assert_node(&mut sender).await;

    connection.abort();
    server.abort();
}

async fn assert_node(sender: &mut hyper::client::conn::http2::SendRequest<Full<Bytes>>) {
    let request = Request::builder()
        .uri("/v1/node/info")
        .header("Authorization", "Bearer secret")
        .body(Full::new(Bytes::new()))
        .expect("request");
    let response = sender.send_request(request).await.expect("response");
    let body = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    assert_eq!(NodeInfo::decode(body).expect("node").node_id, "builder-h2");
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
