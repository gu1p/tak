use std::fs;
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

static ENV_LOCK: Mutex<()> = Mutex::new(());

#[rustfmt::skip]
pub(crate) fn env_lock() -> MutexGuard<'static, ()> { ENV_LOCK.lock().expect("env lock") }

pub(super) fn write_remote_inventory(config_root: &Path, content: &str) {
    let tak_dir = config_root.join("tak");
    fs::create_dir_all(&tak_dir).expect("create config dir");
    fs::write(tak_dir.join("remotes.toml"), content).expect("write inventory");
}

pub(super) struct EnvVarGuard {
    key: &'static str,
    original: Option<String>,
}

#[rustfmt::skip]
impl EnvVarGuard {
    pub(super) fn set(key: &'static str, value: &Path) -> Self { let original = std::env::var(key).ok(); unsafe { std::env::set_var(key, value); } Self { key, original } }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match self.original.as_ref() {
            Some(value) => unsafe {
                std::env::set_var(self.key, value);
            },
            None => unsafe {
                std::env::remove_var(self.key);
            },
        }
    }
}
