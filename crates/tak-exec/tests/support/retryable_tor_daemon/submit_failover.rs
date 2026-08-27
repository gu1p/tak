use super::*;

impl RetryableTorDaemon {
    pub async fn spawn_submit_failover(root: &Path, env: &mut EnvGuard) -> Self {
        let temp = tempfile::tempdir_in(root).expect("submit failover daemon tempdir");
        let socket_path = temp.path().join("takd.sock");
        env.set("TAKD_SOCKET", socket_path.display().to_string());
        let listener = UnixListener::bind(&socket_path).expect("bind submit failover daemon");
        let state = Arc::new(Mutex::new(State {
            failover_results: true,
            submit_failover: true,
            ..State::default()
        }));
        Self::with_listener(temp, listener, state)
    }

    pub async fn spawn_submit_transport_failover(root: &Path, env: &mut EnvGuard) -> Self {
        let temp = tempfile::tempdir_in(root).expect("submit transport failover daemon tempdir");
        let socket_path = temp.path().join("takd.sock");
        env.set("TAKD_SOCKET", socket_path.display().to_string());
        let listener = UnixListener::bind(&socket_path).expect("bind submit transport daemon");
        let state = Arc::new(Mutex::new(State {
            failover_results: true,
            submit_transport_failover: true,
            ..State::default()
        }));
        Self::with_listener(temp, listener, state)
    }

    pub async fn spawn_submit_exhaustion(root: &Path, env: &mut EnvGuard) -> Self {
        let temp = tempfile::tempdir_in(root).expect("submit exhaustion daemon tempdir");
        let socket_path = temp.path().join("takd.sock");
        env.set("TAKD_SOCKET", socket_path.display().to_string());
        let listener = UnixListener::bind(&socket_path).expect("bind submit exhaustion daemon");
        let state = Arc::new(Mutex::new(State {
            failover_results: true,
            submit_always_fails: true,
            ..State::default()
        }));
        Self::with_listener(temp, listener, state)
    }
}
