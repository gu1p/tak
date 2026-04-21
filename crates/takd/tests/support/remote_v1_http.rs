use std::net::SocketAddr;

use prost::Message;
use tak_proto::ErrorResponse;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use takd::daemon::remote::{
    RemoteNodeContext, RemoteRuntimeConfig, SubmitAttemptStore, run_remote_v1_http_server,
};

pub struct RunningServer {
    _temp: tempfile::TempDir,
    pub store: SubmitAttemptStore,
    pub context: RemoteNodeContext,
    pub addr: SocketAddr,
    server: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        self.server.abort();
    }
}

pub struct RawHttpResponse {
    pub head: String,
    pub body: Vec<u8>,
}

pub async fn start_server() -> RunningServer {
    let temp = tempfile::tempdir().expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("takd.sqlite")).expect("store");
    let context = RemoteNodeContext::new(
        node_info(),
        "secret".into(),
        RemoteRuntimeConfig::for_tests(),
    );
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener local addr");
    let server = tokio::spawn(run_remote_v1_http_server(
        listener,
        store.clone(),
        context.clone(),
    ));
    RunningServer {
        _temp: temp,
        store,
        context,
        addr,
        server,
    }
}

pub async fn send_raw_request(addr: SocketAddr, request: &[u8]) -> RawHttpResponse {
    let mut stream = tokio::net::TcpStream::connect(addr)
        .await
        .expect("connect server");
    stream.write_all(request).await.expect("write request");
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
        .expect("response should contain HTTP header terminator");
    RawHttpResponse {
        head: String::from_utf8(response[..split].to_vec()).expect("response utf8"),
        body: response[split..].to_vec(),
    }
}

pub fn decode_error_response(response: &RawHttpResponse) -> ErrorResponse {
    ErrorResponse::decode(response.body.as_slice()).expect("decode error payload")
}

fn node_info() -> tak_proto::NodeInfo {
    tak_proto::NodeInfo {
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
    }
}
