#![allow(dead_code)]

use super::local_daemon_manager::manager_for;
use super::takd_binary::takd_bin;
use std::path::{Path, PathBuf};
use std::sync::{Arc, mpsc};
use tak_core::model::WorkspaceSpec;
use takd::{PeerManager, TorBroker};

mod custom_executable;
mod inventory;
mod runtime;
mod socket_binding;

pub struct LocalDaemonGuard {
    thread: Option<std::thread::JoinHandle<()>>,
    stopped: mpsc::Receiver<Result<(), String>>,
    shutdown: Option<tokio::sync::oneshot::Sender<()>>,
    socket_path: PathBuf,
    db_path: PathBuf,
    _state_dir: tempfile::TempDir,
    socket_binding: socket_binding::SocketBinding,
}
impl LocalDaemonGuard {
    pub fn spawn(socket_path: &Path, spec: &WorkspaceSpec) -> Self {
        Self::spawn_with_broker(socket_path, spec, non_tor_broker())
    }
    pub fn spawn_with_tor_dial_addr(
        socket_path: &Path,
        spec: &WorkspaceSpec,
        dial_addr: String,
    ) -> Self {
        Self::spawn_with_broker(socket_path, spec, TorBroker::for_direct_dial(dial_addr))
    }
    fn spawn_with_broker(socket_path: &Path, spec: &WorkspaceSpec, broker: TorBroker) -> Self {
        let peers = PeerManager::default();
        Self::spawn_inner(socket_path, spec, broker, peers, takd_bin())
    }
    fn spawn_inner(
        socket_path: &Path,
        spec: &WorkspaceSpec,
        broker: TorBroker,
        peers: PeerManager,
        attempt_executable: PathBuf,
    ) -> Self {
        let manager = Arc::clone(&manager_for(spec));
        let socket_path = socket_path.to_path_buf();
        let socket_binding = socket_binding::SocketBinding::new(&socket_path);
        let state_dir = native_state_dir();
        let db_path = state_dir.path().join("run.sqlite");
        let server = runtime::spawn_server(
            socket_binding.server_path().to_path_buf(),
            manager,
            broker,
            peers,
            db_path.clone(),
            attempt_executable,
            state_dir.path().join("remotes.toml"),
        )
        .unwrap_or_else(|error| panic!("local daemon startup failed: {error}"));
        Self {
            thread: Some(server.thread),
            stopped: server.stopped,
            shutdown: Some(server.shutdown),
            socket_path,
            db_path,
            _state_dir: state_dir,
            socket_binding,
        }
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn effective_socket_path(&self) -> &Path {
        self.socket_binding.server_path()
    }
}

fn native_state_dir() -> tempfile::TempDir {
    let root = std::env::var_os("TAK_TEST_TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    std::fs::create_dir_all(&root).expect("create native test temporary directory");
    tempfile::Builder::new()
        .prefix("tak-local-daemon-")
        .tempdir_in(root)
        .expect("create native local daemon state directory")
}

fn non_tor_broker() -> TorBroker {
    TorBroker::for_direct_dial("127.0.0.1:9")
}
