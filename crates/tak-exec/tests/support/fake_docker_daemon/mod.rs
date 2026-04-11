#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tokio::net::UnixListener;
use tokio::sync::Notify;
use tokio::task::JoinHandle;

mod query;
mod request;
mod response;
mod server;
mod tar;
const CONTAINER_ID: &str = "container-123";
const IMAGE_ID: &str = "sha256:test-image";
const LOG_MESSAGE: &[u8] = b"hello from container\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildRecord {
    pub dockerfile: String,
    pub context_entries: Vec<String>,
}
pub struct FakeDockerDaemon {
    socket_path: PathBuf,
    state: Arc<FakeDockerDaemonState>,
    accept_task: JoinHandle<()>,
}
struct FakeDockerDaemonState {
    release_requested: AtomicBool,
    release_notify: Notify,
    builds: Mutex<Vec<BuildRecord>>,
}

impl FakeDockerDaemon {
    pub fn spawn(root: &Path) -> Self {
        let socket_path = root.join("docker.sock");
        if socket_path.exists() {
            std::fs::remove_file(&socket_path).expect("remove stale fake docker socket");
        }

        let listener = UnixListener::bind(&socket_path).expect("bind fake docker socket");
        let state = Arc::new(FakeDockerDaemonState {
            release_requested: AtomicBool::new(false),
            release_notify: Notify::new(),
            builds: Mutex::new(Vec::new()),
        });
        let accept_task =
            tokio::spawn(server::run_fake_docker_daemon(listener, Arc::clone(&state)));

        Self {
            socket_path,
            state,
            accept_task,
        }
    }
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    pub fn release_container_exit(&self) {
        self.state.release_requested.store(true, Ordering::SeqCst);
        self.state.release_notify.notify_waiters();
    }

    pub fn single_build(&self) -> Option<BuildRecord> {
        self.state
            .builds
            .lock()
            .expect("build records lock")
            .first()
            .cloned()
    }
}
impl Drop for FakeDockerDaemon {
    fn drop(&mut self) {
        self.release_container_exit();
        self.accept_task.abort();
        let _ = std::fs::remove_file(&self.socket_path);
    }
}

impl FakeDockerDaemonState {
    async fn wait_until_released(&self) {
        loop {
            if self.release_requested.load(Ordering::SeqCst) {
                return;
            }
            self.release_notify.notified().await;
        }
    }

    fn record_build(&self, build: BuildRecord) {
        self.builds.lock().expect("build records lock").push(build);
    }
}
