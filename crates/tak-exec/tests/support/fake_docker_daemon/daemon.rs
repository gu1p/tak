use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::Ordering;

use tokio::net::UnixListener;
use tokio::task::JoinHandle;

use super::server::run_fake_docker_daemon;
use super::state::FakeDockerDaemonState;
use super::{BuildRecord, CreateRecord, PullRecord};

pub struct FakeDockerDaemon {
    socket_path: PathBuf,
    state: Arc<FakeDockerDaemonState>,
    accept_task: JoinHandle<()>,
}

impl FakeDockerDaemon {
    pub fn spawn(root: &Path) -> Self {
        let socket_path = root.join("docker.sock");
        if socket_path.exists() {
            std::fs::remove_file(&socket_path).expect("remove stale fake docker socket");
        }

        let listener = UnixListener::bind(&socket_path).expect("bind fake docker socket");
        let state = Arc::new(FakeDockerDaemonState::new());
        let accept_task = tokio::spawn(run_fake_docker_daemon(listener, Arc::clone(&state)));
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
        self.state.build_records().first().cloned()
    }

    pub fn build_records(&self) -> Vec<BuildRecord> {
        self.state.build_records()
    }

    pub fn create_records(&self) -> Vec<CreateRecord> {
        self.state.create_records()
    }

    pub fn pull_records(&self) -> Vec<PullRecord> {
        self.state.pull_records()
    }

    pub fn fail_image_removal(&self, status_code: u16) {
        self.state.fail_image_removal(status_code);
    }

    pub fn image_removal_attempts(&self) -> Vec<String> {
        self.state.image_removal_attempts()
    }

    pub fn remove_image(&self, image: &str) {
        self.state.remove_image(image);
    }

    pub fn set_image(&self, image_ref: &str, image_id: &str, size: u64) {
        self.state.set_image(image_ref, image_id, size);
    }
}

impl Drop for FakeDockerDaemon {
    fn drop(&mut self) {
        self.release_container_exit();
        self.accept_task.abort();
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
