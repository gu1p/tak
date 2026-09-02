use super::FakeRunDaemon;

impl Drop for FakeRunDaemon {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}
