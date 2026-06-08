use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::net::UnixListener;
use tokio::task::JoinHandle;

use super::server::run_fake_docker_daemon;
use super::state::FakeDockerDaemonState;
use super::{CreateRecord, FakeDockerConfig};

pub struct FakeDockerDaemon {
    socket_path: PathBuf,
    state: Arc<FakeDockerDaemonState>,
    accept_task: JoinHandle<()>,
}

impl FakeDockerDaemon {
    pub fn spawn(root: &Path, config: FakeDockerConfig) -> Self {
        let socket_path = root.join("docker.sock");
        if socket_path.exists() {
            std::fs::remove_file(&socket_path).expect("remove stale fake docker socket");
        }

        let listener = UnixListener::bind(&socket_path).expect("bind fake docker socket");
        let state = Arc::new(FakeDockerDaemonState::new(
            config.visible_roots,
            config.image_present,
            config.arch,
            config.version_fails,
            config.wait_response_delay,
        ));
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

    pub fn create_records(&self) -> Vec<CreateRecord> {
        self.state.create_records()
    }

    pub fn pull_count(&self) -> u64 {
        self.state.pull_count()
    }

    pub fn removed_containers(&self) -> Vec<String> {
        self.state.removed_containers()
    }

    pub fn add_container(&self, container_id: &str, labels: BTreeMap<String, String>) {
        self.state.add_container(container_id, labels);
    }

    pub fn add_paused_container(&self, container_id: &str, labels: BTreeMap<String, String>) {
        self.state
            .add_container_with_state(container_id, labels, "paused");
    }
}

impl Drop for FakeDockerDaemon {
    fn drop(&mut self) {
        self.accept_task.abort();
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
