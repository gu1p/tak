use tokio::net::TcpListener;

use super::RunningServer;
use takd::daemon::remote::{RemoteNodeContext, run_worker_http_server};

pub async fn restart(server: &mut RunningServer, runtime: takd::RemoteRuntimeConfig) {
    server.server.abort();
    let node = server.context.node_info().unwrap();
    server.context =
        RemoteNodeContext::new(node, "secret".into(), runtime).with_state_root(&server.state_root);
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    server.addr = listener.local_addr().unwrap();
    server.server = tokio::spawn(run_worker_http_server(
        listener,
        server.store.clone(),
        server.context.clone(),
    ));
}
