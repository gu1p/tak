use std::path::{Path, PathBuf};
use std::time::Duration;

use tak_core::model::WorkspaceSpec;
use takd::{PeerManager, TorBroker};

use super::LocalDaemonGuard;

impl LocalDaemonGuard {
    pub fn spawn_with_attempt_executable(
        socket_path: &Path,
        spec: &WorkspaceSpec,
        attempt_executable: PathBuf,
    ) -> Self {
        Self::spawn_inner(
            socket_path,
            spec,
            TorBroker::for_direct_dial("127.0.0.1:9"),
            PeerManager::default(),
            attempt_executable,
        )
    }
}

impl Drop for LocalDaemonGuard {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
        let result = match self.stopped.recv_timeout(Duration::from_secs(5)) {
            Ok(result) => {
                let joined = self.thread.take().expect("daemon thread").join();
                joined
                    .map_err(|_| "server thread panicked".to_owned())
                    .and(result)
            }
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                self.thread.take();
                Err("cooperative shutdown timed out after 5s".to_owned())
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => self
                .thread
                .take()
                .expect("daemon thread")
                .join()
                .map_err(|_| "server thread panicked".to_owned()),
        };
        if result.is_ok() {
            let _ = std::fs::remove_file(&self.socket_path);
        }
        if let Err(error) = result {
            if std::thread::panicking() {
                eprintln!("local daemon shutdown failed during test panic: {error}");
            } else {
                panic!("local daemon shutdown failed: {error}");
            }
        }
    }
}
