use super::tor_probe_retry_policy;
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::Duration;
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
        // SAFETY: tests serialize environment mutation through `ENV_LOCK`.
        unsafe {
            std::env::set_var(key, value);
        }
    }

    fn remove(&mut self, key: &str) {
        self.save(key);
        // SAFETY: tests serialize environment mutation through `ENV_LOCK`.
        unsafe {
            std::env::remove_var(key);
        }
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
                    unsafe {
                        std::env::set_var(key, previous);
                    }
                }
                None => {
                    // SAFETY: tests serialize environment mutation through `ENV_LOCK`.
                    unsafe {
                        std::env::remove_var(key);
                    }
                }
            }
        }
    }
}
#[test]
fn default_tor_probe_retry_policy_allows_two_minutes_for_live_onion_readiness() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.remove("TAK_TOR_PROBE_TIMEOUT_MS");
    env.remove("TAK_TOR_PROBE_BACKOFF_MS");
    env.remove("TAK_TEST_TOR_PROBE_TIMEOUT_MS");
    env.remove("TAK_TEST_TOR_PROBE_BACKOFF_MS");

    let policy = tor_probe_retry_policy();
    assert_eq!(policy.timeout, Duration::from_secs(120));
    assert_eq!(policy.backoff, Duration::from_secs(1));
}
#[test]
fn live_tor_probe_retry_policy_uses_live_env_vars_when_present() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.set("TAK_TOR_PROBE_TIMEOUT_MS", "240000");
    env.set("TAK_TOR_PROBE_BACKOFF_MS", "1500");
    env.remove("TAK_TEST_TOR_PROBE_TIMEOUT_MS");
    env.remove("TAK_TEST_TOR_PROBE_BACKOFF_MS");
    let policy = tor_probe_retry_policy();
    assert_eq!(policy.timeout, Duration::from_secs(240));
    assert_eq!(policy.backoff, Duration::from_millis(1500));
}
#[test]
fn test_tor_probe_retry_policy_overrides_live_env_vars_for_deterministic_tests() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.set("TAK_TOR_PROBE_TIMEOUT_MS", "240000");
    env.set("TAK_TOR_PROBE_BACKOFF_MS", "1500");
    env.set("TAK_TEST_TOR_PROBE_TIMEOUT_MS", "200");
    env.set("TAK_TEST_TOR_PROBE_BACKOFF_MS", "10");
    let policy = tor_probe_retry_policy();
    assert_eq!(policy.timeout, Duration::from_millis(200));
    assert_eq!(policy.backoff, Duration::from_millis(10));
}
