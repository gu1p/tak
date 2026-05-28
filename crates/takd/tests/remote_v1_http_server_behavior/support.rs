use tak_proto::NodeInfo;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use takd::daemon::remote::{
    RemoteNodeContext, RemoteRuntimeConfig, SubmitAttemptStore, run_remote_v1_http_server,
};

pub(super) async fn fetch(path: &str) -> (String, Vec<u8>) {
    let context = RemoteNodeContext::new(
        NodeInfo {
            node_id: "builder-a".into(),
            display_name: "builder-a".into(),
            base_url: "http://127.0.0.1:43123".into(),
            healthy: true,
            pools: vec!["default".into()],
            tags: vec!["builder".into()],
            capabilities: vec!["linux".into()],
            transport: "direct".into(),
            transport_state: "ready".into(),
            transport_detail: String::new(),
        },
        "secret".into(),
        RemoteRuntimeConfig::for_tests(),
    );
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("takd.sqlite"))
        .expect("submit attempt store");
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener local addr");
    let server = tokio::spawn(run_remote_v1_http_server(listener, store, context));
    let mut stream = tokio::net::TcpStream::connect(addr)
        .await
        .expect("connect server");
    stream
        .write_all(request(path).as_bytes())
        .await
        .expect("send request");
    stream.shutdown().await.expect("shutdown write side");

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .expect("read response");
    server.abort();
    split_response(response)
}

fn request(path: &str) -> String {
    format!(
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer secret\r\nConnection: close\r\n\r\n"
    )
}

fn split_response(response: Vec<u8>) -> (String, Vec<u8>) {
    let split = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
        .expect("response should contain header terminator");
    (
        String::from_utf8(response[..split].to_vec()).expect("response utf8"),
        response[split..].to_vec(),
    )
}
