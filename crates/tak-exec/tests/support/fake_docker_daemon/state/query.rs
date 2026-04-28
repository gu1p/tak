use super::FakeDockerDaemonState;
use crate::support::fake_docker_daemon::{BuildRecord, CreateRecord, PullRecord};

impl FakeDockerDaemonState {
    pub(in crate::support::fake_docker_daemon) fn build_records(&self) -> Vec<BuildRecord> {
        self.builds.lock().expect("build records lock").clone()
    }

    pub(in crate::support::fake_docker_daemon) fn create_records(&self) -> Vec<CreateRecord> {
        self.creates.lock().expect("create records lock").clone()
    }

    pub(in crate::support::fake_docker_daemon) fn pull_records(&self) -> Vec<PullRecord> {
        self.pulls.lock().expect("pull records lock").clone()
    }

    pub(in crate::support::fake_docker_daemon) fn image_removal_attempts(&self) -> Vec<String> {
        self.image_removal_attempts
            .lock()
            .expect("image removal attempts lock")
            .clone()
    }
}
