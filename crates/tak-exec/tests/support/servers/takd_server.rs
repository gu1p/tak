#![allow(dead_code)]

use std::path::Path;
use std::sync::OnceLock;

use tak_proto::NodeInfo;
use takd::daemon::remote::{
    RemoteNodeContext, RemoteRuntimeConfig, SubmitAttemptStore, run_remote_v1_http_server,
};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

use crate::support::install_fake_docker;

pub struct RunningTakdServer {
    pub bind_addr: String,
    pub base_url: String,
    pub node_id: String,
    pub bearer_token: String,
    handle: JoinHandle<()>,
}

impl RunningTakdServer {
    pub async fn spawn(node_id: &str, transport: &str, state_root: &Path) -> Self {
        ensure_simulated_container_runtime_env();
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind remote v1 listener");
        let bind_addr = listener.local_addr().expect("listener addr").to_string();
        let base_url = if transport == "tor" {
            format!("http://{node_id}.onion")
        } else {
            format!("http://{bind_addr}")
        };
        let context = RemoteNodeContext::new(
            NodeInfo {
                node_id: node_id.into(),
                display_name: node_id.into(),
                base_url: base_url.clone(),
                healthy: true,
                pools: vec!["build".into()],
                tags: vec!["builder".into()],
                capabilities: vec!["linux".into()],
                transport: transport.into(),
                transport_state: "ready".into(),
                transport_detail: String::new(),
            },
            "secret".into(),
            RemoteRuntimeConfig::for_tests().with_skip_exec_root_probe(true),
        );
        let store = SubmitAttemptStore::with_db_path(state_root.join(format!("{node_id}.sqlite")))
            .expect("submit attempt store");
        let handle = tokio::spawn(async move {
            let _ = run_remote_v1_http_server(listener, store, context).await;
        });
        Self {
            bind_addr,
            base_url,
            node_id: node_id.into(),
            bearer_token: "secret".into(),
            handle,
        }
    }
}

impl Drop for RunningTakdServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}

fn ensure_simulated_container_runtime_env() {
    static FAKE_DOCKER_BIN: OnceLock<std::path::PathBuf> = OnceLock::new();
    let bin_root = FAKE_DOCKER_BIN.get_or_init(|| {
        let path = std::env::temp_dir().join("tak-exec-test-fake-docker");
        install_fake_docker(&path);
        path
    });

    let current_path = std::env::var("PATH").unwrap_or_default();
    let bin_prefix = bin_root.display().to_string();
    if !current_path.split(':').any(|entry| entry == bin_prefix) {
        unsafe { std::env::set_var("PATH", format!("{bin_prefix}:{current_path}")) };
    }
    unsafe { std::env::set_var("TAK_TEST_HOST_PLATFORM", "other") };
}
