use super::*;

pub fn default_socket_path() -> PathBuf {
    tak_core::runtime_paths::default_daemon_socket_path()
}
