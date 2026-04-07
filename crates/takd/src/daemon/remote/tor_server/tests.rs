use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard, OnceLock};

use super::*;

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[test]
fn bind_addr_helpers_trim_and_ignore_empty_values() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.remove("TAKD_REMOTE_V1_BIND_ADDR");
    env.remove("TAKD_TEST_TOR_HS_BIND_ADDR");
    assert_eq!(remote_v1_bind_addr_from_env(), None);
    assert_eq!(test_tor_hidden_service_bind_addr(), None);

    env.set("TAKD_REMOTE_V1_BIND_ADDR", " 127.0.0.1:43123 ");
    env.set("TAKD_TEST_TOR_HS_BIND_ADDR", " 127.0.0.1:43124 ");
    assert_eq!(
        remote_v1_bind_addr_from_env().as_deref(),
        Some("127.0.0.1:43123")
    );
    assert_eq!(
        test_tor_hidden_service_bind_addr().as_deref(),
        Some("127.0.0.1:43124")
    );
}

#[test]
fn runtime_config_reads_defaults_and_explicit_overrides() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.remove("TAKD_TOR_HS_NICKNAME");
    env.remove("TAKD_TOR_STATE_DIR");
    env.remove("TAKD_TOR_CACHE_DIR");
    assert!(
        tor_hidden_service_runtime_config_from_env()
            .expect("config")
            .is_none()
    );

    env.set("TAKD_TOR_HS_NICKNAME", "buildertor");
    let default_config = tor_hidden_service_runtime_config_from_env()
        .expect("default config")
        .expect("runtime config");
    assert_eq!(default_config.nickname, "buildertor");
    assert!(default_config.state_dir.ends_with("takd-arti-state"));
    assert!(default_config.cache_dir.ends_with("takd-arti-cache"));

    env.set("TAKD_TOR_STATE_DIR", "/tmp/custom-state");
    env.set("TAKD_TOR_CACHE_DIR", "/tmp/custom-cache");
    let override_config = tor_hidden_service_runtime_config_from_env()
        .expect("override config")
        .expect("runtime config");
    assert_eq!(
        override_config.state_dir,
        PathBuf::from("/tmp/custom-state")
    );
    assert_eq!(
        override_config.cache_dir,
        PathBuf::from("/tmp/custom-cache")
    );
}

#[test]
fn hidden_service_config_validates_nicknames() {
    assert!(build_tor_hidden_service_config("buildertor").is_ok());
    let err = build_tor_hidden_service_config("bad nickname").expect_err("invalid nickname");
    assert!(format!("{err:#}").contains("invalid tor hidden-service nickname"));
}

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
        // SAFETY: tests serialize environment mutation through `ENV_LOCK`.
        unsafe { std::env::set_var(key, value) };
    }

    fn remove(&mut self, key: &str) {
        self.save(key);
        // SAFETY: tests serialize environment mutation through `ENV_LOCK`.
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
                Some(previous) => {
                    // SAFETY: tests serialize environment mutation through `ENV_LOCK`.
                    unsafe { std::env::set_var(key, previous) }
                }
                None => {
                    // SAFETY: tests serialize environment mutation through `ENV_LOCK`.
                    unsafe { std::env::remove_var(key) }
                }
            }
        }
    }
}
