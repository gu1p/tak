#![cfg(test)]

use std::time::Duration;

use crate::test_env::{EnvGuard, env_lock};

use super::{startup_session_timeout, tor_recovery_config};

#[test]
fn default_startup_session_timeout_matches_live_tor_probe_budget() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.remove("TAKD_TOR_STARTUP_SESSION_TIMEOUT_MS");

    assert_eq!(
        startup_session_timeout(Duration::from_secs(300)),
        Duration::from_secs(300)
    );
}

#[test]
fn default_recovery_probe_timeout_matches_live_tor_probe_budget() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.remove("TAKD_TOR_RECOVERY_PROBE_TIMEOUT_MS");

    assert_eq!(
        tor_recovery_config().probe_timeout,
        Duration::from_secs(300)
    );
}
