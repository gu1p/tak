use super::FakeDockerDaemon;

impl Drop for FakeDockerDaemon {
    fn drop(&mut self) {
        self.release_container_exit();
        self.accept_task.abort();
        let _ = std::fs::remove_file(&self.socket_path);
    }
}
