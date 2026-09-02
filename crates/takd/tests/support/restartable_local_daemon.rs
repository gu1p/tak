use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use tempfile::TempDir;

pub struct RestartableLocalDaemon {
    _temp: TempDir,
    config_root: PathBuf,
    state_root: PathBuf,
    runtime_root: PathBuf,
    config_home: PathBuf,
    socket: PathBuf,
    child: Child,
}

impl RestartableLocalDaemon {
    pub fn start() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let config_root = temp.path().join("missing-agent-config");
        let state_root = temp.path().join("state");
        let runtime_root = temp.path().join("runtime");
        let config_home = temp.path().join("config-home");
        let socket = runtime_root.join("tak/takd.sock");
        let child = spawn(&config_root, &state_root, &runtime_root, &config_home);
        wait_ready(&socket);
        Self {
            _temp: temp,
            config_root,
            state_root,
            runtime_root,
            config_home,
            socket,
            child,
        }
    }

    pub fn crash_and_restart(&mut self) {
        self.child.kill().unwrap();
        self.child.wait().unwrap();
        self.child = spawn(
            &self.config_root,
            &self.state_root,
            &self.runtime_root,
            &self.config_home,
        );
        wait_ready(&self.socket);
    }

    pub fn state_root(&self) -> &Path {
        &self.state_root
    }

    pub fn scratch_path(&self, name: &str) -> PathBuf {
        self._temp.path().join(name)
    }
}

impl Drop for RestartableLocalDaemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

fn spawn(config: &Path, state: &Path, runtime: &Path, config_home: &Path) -> Child {
    let command_root = state.parent().expect("daemon root parent");
    let runtime = runtime
        .strip_prefix(command_root)
        .expect("relative runtime");
    Command::new(super::takd_bin())
        .args(["serve", "--config-root"])
        .arg(config)
        .arg("--state-root")
        .arg(state)
        .current_dir(command_root)
        .env("XDG_RUNTIME_DIR", runtime)
        .env("XDG_CONFIG_HOME", config_home)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

fn wait_ready(socket: &Path) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let connection_path = super::socket_path::bind_path(socket);
    while UnixStream::connect(&connection_path).is_err() {
        assert!(Instant::now() < deadline, "takd socket was not ready");
        std::thread::sleep(Duration::from_millis(20));
    }
}
