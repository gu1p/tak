use super::*;

pub(crate) async fn run_fake_docker_daemon(
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
