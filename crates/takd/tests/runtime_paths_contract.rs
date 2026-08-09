use std::path::PathBuf;

use takd::default_socket_path;

use crate::support;

use support::env::{EnvGuard, env_lock};

#[test]
fn default_socket_path_follows_xdg_runtime_directory_when_present() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.set("XDG_RUNTIME_DIR", "/tmp/tak-runtime");
    assert_eq!(
        default_socket_path(),
        PathBuf::from("/tmp/tak-runtime/tak/takd.sock")
    );
}

#[test]
fn default_socket_path_falls_back_when_xdg_runtime_directory_is_missing() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.remove("XDG_RUNTIME_DIR");
    assert!(
        default_socket_path()
            .to_string_lossy()
            .starts_with("/tmp/tak-")
    );
}
