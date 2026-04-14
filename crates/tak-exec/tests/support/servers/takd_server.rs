#![allow(dead_code)]

use std::path::Path;

use tak_proto::NodeInfo;
use takd::daemon::remote::{RemoteNodeContext, SubmitAttemptStore, run_remote_v1_http_server};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

pub struct RunningTakdServer {
    pub bind_addr: String,
    pub base_url: String,
    pub node_id: String,
    pub bearer_token: String,
    handle: JoinHandle<()>,
}

impl RunningTakdServer {
    pub async fn spawn(node_id: &str, transport: &str, state_root: &Path) -> Self {
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
