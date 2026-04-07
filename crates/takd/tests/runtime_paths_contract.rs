use std::path::PathBuf;

use takd::{default_socket_path, default_state_db_path};

mod support;

use support::env::{EnvGuard, env_lock};

#[test]
fn default_paths_follow_xdg_directories_when_present() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.set("XDG_RUNTIME_DIR", "/tmp/tak-runtime");
    env.set("XDG_STATE_HOME", "/tmp/tak-state");
    assert_eq!(
        default_socket_path(),
        PathBuf::from("/tmp/tak-runtime/tak/takd.sock")
    );
    assert_eq!(
        default_state_db_path(),
        PathBuf::from("/tmp/tak-state/tak/takd.sqlite")
    );
}

#[test]
fn default_paths_fall_back_when_xdg_variables_are_missing() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.remove("XDG_RUNTIME_DIR");
    env.remove("XDG_STATE_HOME");
    env.set("HOME", "/tmp/tak-home");
    assert!(
        default_socket_path()
            .to_string_lossy()
            .starts_with("/tmp/tak-")
    );
    assert_eq!(
        default_state_db_path(),
        PathBuf::from("/tmp/tak-home/.local/state/tak/takd.sqlite")
    );
    env.remove("HOME");
    assert_eq!(default_state_db_path(), PathBuf::from("/tmp/takd.sqlite"));
}
