use std::time::Duration;

use crate::test_env::{EnvGuard, env_lock};

use super::startup_probe_retry_policy;

#[test]
fn default_startup_probe_retry_policy_allows_two_minutes_for_live_hidden_service_readiness() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.remove("TAKD_TOR_STARTUP_PROBE_TIMEOUT_MS");
    env.remove("TAKD_TOR_STARTUP_PROBE_BACKOFF_MS");

    let policy = startup_probe_retry_policy();

    assert_eq!(policy.timeout, Duration::from_secs(120));
    assert_eq!(policy.backoff, Duration::from_secs(1));
}

#[test]
fn startup_probe_retry_policy_uses_live_env_vars_when_present() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.set("TAKD_TOR_STARTUP_PROBE_TIMEOUT_MS", "300000");
    env.set("TAKD_TOR_STARTUP_PROBE_BACKOFF_MS", "1500");

    let policy = startup_probe_retry_policy();

    assert_eq!(policy.timeout, Duration::from_secs(300));
    assert_eq!(policy.backoff, Duration::from_millis(1500));
}
