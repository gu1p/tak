use std::path::Path;
use std::sync::Arc;

use tokio::net::UnixListener;
use tokio::sync::Mutex;

use super::EnvGuard;

mod io;
mod responses;
mod state;
mod submit_failover;

pub(super) use state::State;

pub struct RetryableTorDaemon {
    state: Arc<Mutex<State>>,
    _temp: tempfile::TempDir,
    task: tokio::task::JoinHandle<()>,
}

impl RetryableTorDaemon {
    pub async fn spawn(root: &Path, env: &mut EnvGuard) -> Self {
        let temp = tempfile::tempdir_in(root).expect("retryable daemon tempdir");
        let socket_path = temp.path().join("takd.sock");
        env.set("TAKD_SOCKET", socket_path.display().to_string());
        let listener = UnixListener::bind(&socket_path).expect("bind retryable fake daemon");
        let state = Arc::new(Mutex::new(State {
            upload_failover: true,
            ..State::default()
        }));
        Self::with_listener(temp, listener, state)
    }

    pub async fn spawn_non_retryable(root: &Path, env: &mut EnvGuard) -> Self {
        let temp = tempfile::tempdir_in(root).expect("non-retryable daemon tempdir");
        let socket_path = temp.path().join("takd.sock");
        env.set("TAKD_SOCKET", socket_path.display().to_string());
        let listener = UnixListener::bind(&socket_path).expect("bind fake daemon");
        let state = Arc::new(Mutex::new(State {
            non_retryable_peers: true,
            ..State::default()
        }));
        Self::with_listener(temp, listener, state)
    }

    pub async fn spawn_failover(root: &Path, env: &mut EnvGuard) -> Self {
        let temp = tempfile::tempdir_in(root).expect("failover daemon tempdir");
        let socket_path = temp.path().join("takd.sock");
        env.set("TAKD_SOCKET", socket_path.display().to_string());
        let listener = UnixListener::bind(&socket_path).expect("bind failover daemon");
        let state = Arc::new(Mutex::new(State {
            failover_results: true,
            ..State::default()
        }));
        Self::with_listener(temp, listener, state)
    }

    fn with_listener(
        temp: tempfile::TempDir,
        listener: UnixListener,
        state: Arc<Mutex<State>>,
    ) -> Self {
        let task = tokio::spawn(io::serve(listener, Arc::clone(&state)));
        Self {
            state,
            _temp: temp,
            task,
        }
    }

    pub async fn submit_attempts(&self) -> Vec<u32> {
        self.state.lock().await.submit_attempts.clone()
    }

    pub async fn stream_offsets(&self) -> Vec<u64> {
        self.state.lock().await.stream_offsets.clone()
    }

    pub async fn distinct_upload_ids(&self) -> usize {
        let mut ids = self.state.lock().await.upload_ids.clone();
        ids.sort();
        ids.dedup();
        ids.len()
    }

    pub async fn peer_requests(&self) -> u32 {
        self.state.lock().await.peer_requests
    }

    pub async fn placement_exclusions(&self) -> Vec<Vec<String>> {
        self.state.lock().await.placement_exclusions.clone()
    }
}

impl Drop for RetryableTorDaemon {
    fn drop(&mut self) {
        self.task.abort();
    }
}
