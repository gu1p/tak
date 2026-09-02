use std::path::{Path, PathBuf};

pub struct LiveDirectRoots {
    pub server_config_root: PathBuf,
    pub server_state_root: PathBuf,
    pub client_config_root: PathBuf,
}

impl LiveDirectRoots {
    pub fn new(base: &Path) -> Self {
        let current = std::env::current_dir().expect("resolve test working directory");
        let absolute = if base.is_absolute() {
            base.to_path_buf()
        } else {
            current.join(base)
        };
        Self {
            server_config_root: absolute.join("server-config"),
            server_state_root: absolute.join("server-state"),
            client_config_root: absolute.join("client-config"),
        }
    }

    pub fn service_log_path(&self) -> PathBuf {
        self.server_state_root.join("service.log")
    }
}
