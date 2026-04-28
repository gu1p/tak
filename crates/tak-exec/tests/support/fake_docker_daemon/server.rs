use std::sync::Arc;

use tokio::net::UnixListener;

use super::connection::handle_connection;
use super::state::FakeDockerDaemonState;

pub(super) async fn run_fake_docker_daemon(
    listener: UnixListener,
    state: Arc<FakeDockerDaemonState>,
) {
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            break;
        };
        let state = Arc::clone(&state);
        tokio::spawn(async move {
            let _ = handle_connection(stream, state).await;
        });
    }
}
