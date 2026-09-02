use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use tokio::net::UnixListener;
use tokio::task::JoinHandle;

use super::server::run_fake_docker_daemon;
use super::state::FakeDockerDaemonState;
use super::{CreateRecord, DockerOperation, FakeDockerConfig};

pub struct FakeDockerDaemon {
    requested_socket_path: PathBuf,
    socket_path: PathBuf,
    state: Arc<FakeDockerDaemonState>,
    accept_task: JoinHandle<()>,
}

impl FakeDockerDaemon {
    pub fn spawn(root: &Path, config: FakeDockerConfig) -> Self {
        let requested_socket_path = root.join("docker.sock");
        if requested_socket_path.exists() {
            std::fs::remove_file(&requested_socket_path).expect("remove stale fake docker socket");
        }

        let socket_path = super::super::socket_path::bind_path(&requested_socket_path);
        let listener = UnixListener::bind(&socket_path).expect("bind fake docker socket");
        let state = Arc::new(FakeDockerDaemonState::new(
            config.visible_roots,
            config.image_present,
            config.arch,
            config.version_fails,
            config.wait_response_delay,
            config.ping_response_delay,
            config.memory_usage_bytes,
            config.removal_failures,
            config.oom_killed,
        ));
        let accept_task = tokio::spawn(run_fake_docker_daemon(listener, Arc::clone(&state)));

        Self {
            requested_socket_path,
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

    pub fn operations(&self) -> Vec<DockerOperation> {
        self.state.operations()
    }

    pub fn add_container(&self, container_id: &str, labels: BTreeMap<String, String>) {
        self.state.add_container(container_id, labels);
    }

    pub fn add_paused_container(&self, container_id: &str, labels: BTreeMap<String, String>) {
        self.state
            .add_container_with_state(container_id, labels, "paused");
    }

    pub fn pause_container_after_next_list(&self, container_id: &str) {
        self.state.pause_container_after_next_list(container_id);
    }
}

impl Drop for FakeDockerDaemon {
    fn drop(&mut self) {
        self.accept_task.abort();
        let _ = std::fs::remove_file(&self.requested_socket_path);
    }
}
