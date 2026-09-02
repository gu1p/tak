use std::net::SocketAddr;

use tokio::net::TcpListener;

use takd::daemon::remote::{RemoteNodeContext, SubmitAttemptStore, run_worker_http_server};

mod raw;
mod restart;

pub use raw::{RawHttpResponse, decode_error_response, send_raw_request};
pub use restart::restart;

pub struct RunningServer {
    _temp: tempfile::TempDir,
    pub state_root: std::path::PathBuf,
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

pub async fn start_server() -> RunningServer {
    start_server_for_node("builder-a").await
}

pub async fn start_server_for_node(node_id: &str) -> RunningServer {
    start_server_for_node_with_runtime(node_id, super::runtime_config::isolated()).await
}

pub async fn start_server_with_runtime(runtime: takd::RemoteRuntimeConfig) -> RunningServer {
    start_server_for_node_with_options("builder-a", runtime, None).await
}

pub async fn start_server_with_runtime_and_image_cache(
    runtime: takd::RemoteRuntimeConfig,
    image_cache: takd::RemoteImageCacheRuntimeConfig,
) -> RunningServer {
    start_server_for_node_with_options("builder-a", runtime, Some(image_cache)).await
}

async fn start_server_for_node_with_runtime(
    node_id: &str,
    runtime: takd::RemoteRuntimeConfig,
) -> RunningServer {
    start_server_for_node_with_options(node_id, runtime, None).await
}

async fn start_server_for_node_with_options(
    node_id: &str,
    runtime: takd::RemoteRuntimeConfig,
    image_cache: Option<takd::RemoteImageCacheRuntimeConfig>,
) -> RunningServer {
    std::fs::create_dir_all(".tmp").expect("create test temp root");
    let temp = tempfile::tempdir_in(".tmp").expect("tempdir");
    let store = SubmitAttemptStore::with_db_path(temp.path().join("takd.sqlite")).expect("store");
    let mut context = RemoteNodeContext::new(node_info(node_id), "secret".into(), runtime)
        .with_state_root(temp.path());
    if let Some(image_cache) = image_cache {
        context = context.with_image_cache_config(image_cache);
    }
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind listener");
    let addr = listener.local_addr().expect("listener local addr");
    let server = tokio::spawn(run_worker_http_server(
        listener,
        store.clone(),
        context.clone(),
    ));
    RunningServer {
        state_root: temp.path().to_path_buf(),
        _temp: temp,
        store,
        context,
        addr,
        server,
    }
}

fn node_info(node_id: &str) -> tak_proto::NodeInfo {
    tak_proto::NodeInfo {
        node_id: node_id.into(),
        display_name: node_id.into(),
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
