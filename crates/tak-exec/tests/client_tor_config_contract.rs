#![allow(clippy::await_holding_lock)]

mod support;

use support::{EnvGuard, env_lock};

#[test]
fn default_client_tor_config_uses_xdg_state_home_when_present() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    env.set("XDG_STATE_HOME", temp.path().display().to_string());
    env.remove("HOME");

    let config =
        tak_exec::default_client_tor_config().expect("tak client tor config from XDG_STATE_HOME");
    assert_serialized_config_contains(
        &config,
        &temp.path().join("tak/arti/state").display().to_string(),
    );
    assert_serialized_config_contains(
        &config,
        &temp.path().join("tak/arti/cache").display().to_string(),
    );
}

#[test]
fn default_client_tor_config_falls_back_to_home_local_state() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let home = tempfile::tempdir().expect("home");
    env.remove("XDG_STATE_HOME");
    env.set("HOME", home.path().display().to_string());

    let config =
        tak_exec::default_client_tor_config().expect("tak client tor config from HOME fallback");
    assert_serialized_config_contains(
        &config,
        &home
            .path()
            .join(".local/state/tak/arti/state")
            .display()
            .to_string(),
    );
    assert_serialized_config_contains(
        &config,
        &home
            .path()
            .join(".local/state/tak/arti/cache")
            .display()
            .to_string(),
    );
}

#[test]
fn default_client_tor_config_requires_xdg_state_home_or_home() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    env.remove("XDG_STATE_HOME");
    env.remove("HOME");

    let error = tak_exec::default_client_tor_config().expect_err("missing state roots should fail");
    assert!(
        error
            .to_string()
            .contains("failed to resolve tak client state root"),
        "unexpected error: {error:#}"
    );
}

fn assert_serialized_config_contains(config: &arti_client::TorClientConfig, needle: &str) {
    let serialized = format!("{config:#?}");
    assert!(
        serialized.contains(needle),
        "serialized config should contain `{needle}`:\n{serialized}"
    );
}
