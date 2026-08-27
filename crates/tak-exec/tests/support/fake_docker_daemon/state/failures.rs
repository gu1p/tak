use super::FakeDockerDaemonState;

impl FakeDockerDaemonState {
    pub(in crate::support::fake_docker_daemon) fn fail_build(&self, message: &str) {
        *self
            .build_failure_message
            .lock()
            .expect("build failure lock") = Some(message.to_string());
    }

    pub(in crate::support::fake_docker_daemon) fn build_failure_message(&self) -> Option<String> {
        self.build_failure_message
            .lock()
            .expect("build failure lock")
            .clone()
    }

    pub(in crate::support::fake_docker_daemon) fn fail_container_removal(&self, message: &str) {
        *self
            .container_removal_failure_message
            .lock()
            .expect("container removal failure lock") = Some(message.to_string());
    }

    pub(in crate::support::fake_docker_daemon) fn container_removal_failure_message(
        &self,
    ) -> Option<String> {
        self.container_removal_failure_message
            .lock()
            .expect("container removal failure lock")
            .clone()
    }

    pub(in crate::support::fake_docker_daemon) fn fail_start(&self, message: &str) {
        *self
            .start_failure_message
            .lock()
            .expect("start failure lock") = Some(message.to_string());
    }

    pub(in crate::support::fake_docker_daemon) fn start_failure_message(&self) -> Option<String> {
        self.start_failure_message
            .lock()
            .expect("start failure lock")
            .clone()
    }

    pub(in crate::support::fake_docker_daemon) fn fail_logs(&self, message: &str) {
        *self.logs_failure_message.lock().expect("logs failure lock") = Some(message.to_string());
    }

    pub(in crate::support::fake_docker_daemon) fn logs_failure_message(&self) -> Option<String> {
        self.logs_failure_message
            .lock()
            .expect("logs failure lock")
            .clone()
    }
}
