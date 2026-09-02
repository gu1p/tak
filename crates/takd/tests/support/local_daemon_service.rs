use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::thread;
use std::time::Duration;

use tempfile::TempDir;

pub struct LocalDaemonService {
    _temp: TempDir,
    state_root: PathBuf,
    socket: PathBuf,
    child: Child,
}

impl LocalDaemonService {
    pub fn start() -> Self {
        let temp = tempfile::tempdir().expect("tempdir");
        let config_root = temp.path().join("config");
        let state_root = temp.path().join("state");
        let runtime_root = temp.path().join("runtime");
        let paths = super::daemon_command_paths::DaemonCommandPaths::new(&config_root, &state_root);
        let init = paths
            .rooted_command(&super::takd_bin(), "init")
            .args([
                "--node-id",
                "v2-log-contract",
                "--transport",
                "direct",
                "--base-url",
                "http://127.0.0.1:0",
            ])
            .output()
            .expect("run takd init");
        assert!(init.status.success(), "takd init should succeed");

        let child = paths
            .rooted_command(&super::takd_bin(), "serve")
            .env("XDG_RUNTIME_DIR", paths.runtime_root())
            .env("TAKD_REMOTE_EXEC_ROOT", paths.remote_exec_root())
            .env("RUST_LOG", "debug")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn takd serve");
        Self {
            socket: runtime_root.join("tak/takd.sock"),
            _temp: temp,
            state_root,
            child,
        }
    }

    pub fn exchange(&self, frame: &str) -> String {
        let mut stream = connect(&self.socket);
        writeln!(stream, "{frame}").expect("write request");
        let mut response = String::new();
        BufReader::new(stream)
            .read_line(&mut response)
            .expect("read response");
        assert!(!response.is_empty(), "daemon should return a response");
        response
    }

    pub fn service_log(&self) -> String {
        fs::read_to_string(self.state_root.join("service.log")).expect("read service log")
    }
}

impl Drop for LocalDaemonService {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn connect(socket: &Path) -> UnixStream {
    let connection_path = super::socket_path::bind_path(socket);
    for _ in 0..100 {
        if let Ok(stream) = UnixStream::connect(&connection_path) {
            return stream;
        }
        thread::sleep(Duration::from_millis(20));
    }
    panic!("timed out connecting to {}", socket.display());
}
