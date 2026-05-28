use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use tak_core::runtime_paths::{
    daemon_socket_parent_requires_owner_only, default_daemon_socket_path,
};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn env_lock() -> MutexGuard<'static, ()> {
    match ENV_LOCK.get_or_init(|| Mutex::new(())).lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

#[derive(Default)]
struct EnvGuard {
    saved: Vec<(String, Option<String>)>,
}

impl EnvGuard {
    fn set(&mut self, key: &str, value: &str) {
        self.save(key);
        unsafe { std::env::set_var(key, value) };
    }

    fn remove(&mut self, key: &str) {
        self.save(key);
        unsafe { std::env::remove_var(key) };
    }

    fn save(&mut self, key: &str) {
        if !self.saved.iter().any(|(saved_key, _)| saved_key == key) {
            self.saved.push((key.to_string(), std::env::var(key).ok()));
        }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.saved.iter().rev() {
            match value {
                Some(previous) => unsafe { std::env::set_var(key, previous) },
                None => unsafe { std::env::remove_var(key) },
            }
        }
    }
}

#[test]
fn daemon_socket_path_follows_xdg_runtime_dir_when_present() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.set("XDG_RUNTIME_DIR", "/tmp/tak-runtime");

    assert_eq!(
        default_daemon_socket_path(),
        PathBuf::from("/tmp/tak-runtime/tak/takd.sock")
    );
}

#[cfg(unix)]
#[test]
fn daemon_socket_path_fallback_is_stable_across_processes() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.remove("XDG_RUNTIME_DIR");

    let uid = unsafe { libc::geteuid() };
    assert_eq!(
        default_daemon_socket_path(),
        PathBuf::from(format!("/tmp/tak-{uid}/takd.sock"))
    );
}

#[cfg(unix)]
#[test]
fn fallback_daemon_socket_parent_requires_owner_only_permissions() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.remove("XDG_RUNTIME_DIR");

    assert!(daemon_socket_parent_requires_owner_only(
        &default_daemon_socket_path()
    ));
    assert!(!daemon_socket_parent_requires_owner_only(Path::new(
        "/tmp/custom-takd.sock"
    )));
}
