use crate::support;

use std::time::Duration;

use support::env::{EnvGuard, env_lock};
use support::live_tor_http::live_tor_test_timeout;

fn repo_root() -> &'static std::path::Path {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
}

#[test]
fn live_tor_http_timeout_defaults_to_three_minutes() {
    let _lock = env_lock();
    let mut env = EnvGuard::default();
    env.remove("TAK_LIVE_TOR_TEST_TIMEOUT_SECS");

    assert_eq!(live_tor_test_timeout(), Duration::from_secs(180));
}

#[test]
fn takd_live_tor_harness_extends_startup_session_timeout() -> anyhow::Result<()> {
    let source =
        std::fs::read_to_string(repo_root().join("crates/takd/tests/support/live_tor_cli/mod.rs"))?;

    assert!(
        source.contains("TAKD_TOR_STARTUP_SESSION_TIMEOUT_MS"),
        "takd live Tor harness must extend the startup session timeout, not just the probe timeout:\n{source}"
    );
    Ok(())
}

#[test]
fn live_tor_http_timeout_uses_env_override() {
    let _lock = env_lock();
    let mut env = EnvGuard::default();
    env.set("TAK_LIVE_TOR_TEST_TIMEOUT_SECS", "420");

    assert_eq!(live_tor_test_timeout(), Duration::from_secs(420));
}
