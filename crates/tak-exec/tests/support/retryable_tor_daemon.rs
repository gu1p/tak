use std::path::Path;
use std::sync::Arc;

use tokio::net::UnixListener;
use tokio::sync::Mutex;

use super::EnvGuard;

mod io;
mod responses;

pub struct RetryableTorDaemon {
    state: Arc<Mutex<State>>,
    _temp: tempfile::TempDir,
    task: tokio::task::JoinHandle<()>,
}

#[derive(Default)]
pub(super) struct State {
    pub(super) non_retryable_peers: bool,
    pub(super) peer_requests: u32,
    pub(super) committed: u64,
    pub(super) size: u64,
    pub(super) drops_at_committed_offset: u8,
    pub(super) upload_ids: Vec<String>,
    pub(super) stream_offsets: Vec<u64>,
    pub(super) submit_attempts: Vec<u32>,
}

impl RetryableTorDaemon {
    pub async fn spawn(root: &Path, env: &mut EnvGuard) -> Self {
        let temp = tempfile::tempdir_in(root).expect("retryable daemon tempdir");
        let socket_path = temp.path().join("takd.sock");
        env.set("TAKD_SOCKET", socket_path.display().to_string());
        let listener = UnixListener::bind(&socket_path).expect("bind retryable fake daemon");
        let state = Arc::new(Mutex::new(State::default()));
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
}

impl Drop for RetryableTorDaemon {
    fn drop(&mut self) {
        self.task.abort();
    }
}
