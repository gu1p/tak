use std::path::{Path, PathBuf};

pub fn default_daemon_socket_path() -> PathBuf {
    if let Some(runtime) = non_empty_env("XDG_RUNTIME_DIR") {
        return Path::new(&runtime).join("tak/takd.sock");
    }
    fallback_daemon_socket_path()
}

pub fn daemon_socket_parent_requires_owner_only(socket_path: &Path) -> bool {
    fallback_daemon_socket_parent_requires_owner_only(socket_path)
}

#[cfg(unix)]
fn fallback_daemon_socket_path() -> PathBuf {
    let uid = unsafe { libc::geteuid() };
    PathBuf::from(format!("/tmp/tak-{uid}/takd.sock"))
}

#[cfg(unix)]
fn fallback_daemon_socket_parent_requires_owner_only(socket_path: &Path) -> bool {
    socket_path == fallback_daemon_socket_path()
}

#[cfg(not(unix))]
fn fallback_daemon_socket_path() -> PathBuf {
    PathBuf::from("/tmp/tak/takd.sock")
}

#[cfg(not(unix))]
fn fallback_daemon_socket_parent_requires_owner_only(_socket_path: &Path) -> bool {
    false
}

fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
