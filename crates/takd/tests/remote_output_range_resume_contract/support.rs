use std::fs;
use std::path::Path;

use takd::{RemoteNodeContext, RemoteRuntimeConfig, SubmitAttemptStore};

pub(super) struct RangeFixture {
    pub(super) addr: std::net::SocketAddr,
    pub(super) store: SubmitAttemptStore,
    pub(super) exec_root_base: std::path::PathBuf,
    server: tokio::task::JoinHandle<anyhow::Result<()>>,
}

impl RangeFixture {
    pub(super) async fn spawn(temp: &tempfile::TempDir) -> Self {
        let exec_root_base = temp.path().join("takd-remote-exec");
        let store =
            SubmitAttemptStore::with_db_path(temp.path().join("takd.sqlite")).expect("store");
        let context = RemoteNodeContext::new(
            node_info(),
            "secret".into(),
            RemoteRuntimeConfig::for_tests().with_explicit_remote_exec_root(exec_root_base.clone()),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("addr");
        let server = tokio::spawn(takd::run_remote_v1_http_server(
            listener,
            store.clone(),
            context,
        ));
        Self {
            addr,
            store,
            exec_root_base,
            server,
        }
    }
}

impl Drop for RangeFixture {
    fn drop(&mut self) {
        self.server.abort();
    }
}

pub(super) fn write_artifact(temp: &tempfile::TempDir, key: &str) {
    let artifact_root = temp
        .path()
        .join("takd-remote-artifacts")
        .join(key.replace(':', "_"));
    fs::create_dir_all(&artifact_root).expect("artifact root");
    fs::write(artifact_root.join("out.txt"), b"hello resumable output").expect("artifact");
}

pub(super) fn register_submit(store: &SubmitAttemptStore, exec_root_base: &Path) -> String {
    match store
        .register_submit("run-range", Some(1), "builder-a", exec_root_base)
        .expect("register submit")
    {
        takd::SubmitRegistration::Created { idempotency_key }
        | takd::SubmitRegistration::Attached { idempotency_key } => idempotency_key,
    }
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
