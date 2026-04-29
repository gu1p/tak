#![cfg(test)]

use std::time::Duration;

use crate::test_env::{EnvGuard, env_lock};

use super::{CappedExponentialBackoff, startup_probe_retry_policy};

#[test]
fn default_startup_probe_policy_uses_capped_exponential_backoff() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.remove("TAKD_TOR_STARTUP_PROBE_TIMEOUT_MS");
    env.remove("TAKD_TOR_STARTUP_PROBE_BACKOFF_MS");
    env.remove("TAKD_TOR_STARTUP_PROBE_MAX_BACKOFF_MS");

    let policy = startup_probe_retry_policy();
    let mut backoff = CappedExponentialBackoff::new(policy.initial_backoff, policy.max_backoff);

    assert_eq!(policy.timeout, Duration::from_secs(120));
    assert_eq!(backoff.next_backoff(), Duration::from_secs(1));
    assert_eq!(backoff.next_backoff(), Duration::from_secs(2));
    assert_eq!(backoff.next_backoff(), Duration::from_secs(4));
    assert_eq!(backoff.next_backoff(), Duration::from_secs(8));
    assert_eq!(backoff.next_backoff(), Duration::from_secs(15));
    assert_eq!(backoff.next_backoff(), Duration::from_secs(15));
}

#[test]
fn startup_probe_policy_uses_live_backoff_env_vars_when_present() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.set("TAKD_TOR_STARTUP_PROBE_TIMEOUT_MS", "300000");
    env.set("TAKD_TOR_STARTUP_PROBE_BACKOFF_MS", "1500");
    env.set("TAKD_TOR_STARTUP_PROBE_MAX_BACKOFF_MS", "5000");

    let policy = startup_probe_retry_policy();
    let mut backoff = CappedExponentialBackoff::new(policy.initial_backoff, policy.max_backoff);

    assert_eq!(policy.timeout, Duration::from_secs(300));
    assert_eq!(backoff.next_backoff(), Duration::from_millis(1500));
    assert_eq!(backoff.next_backoff(), Duration::from_millis(3000));
    assert_eq!(backoff.next_backoff(), Duration::from_millis(5000));
    assert_eq!(backoff.next_backoff(), Duration::from_millis(5000));
}
