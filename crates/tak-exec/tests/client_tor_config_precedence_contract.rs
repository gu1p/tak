#![allow(clippy::await_holding_lock)]

mod support;

use support::{EnvGuard, env_lock};

#[test]
fn default_client_tor_config_prefers_xdg_state_home_over_home() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let xdg_state_home = tempfile::tempdir().expect("xdg state home");
    let home = tempfile::tempdir().expect("home");
    env.set(
        "XDG_STATE_HOME",
        xdg_state_home.path().display().to_string(),
    );
    env.set("HOME", home.path().display().to_string());

    let config =
        tak_exec::default_client_tor_config().expect("tak client tor config prefers xdg state");
    let serialized = format!("{config:#?}");
    assert!(
        serialized.contains(
            &xdg_state_home
                .path()
                .join("tak/arti/state")
                .display()
                .to_string()
        ),
        "serialized config should use XDG_STATE_HOME: {serialized}"
    );
    assert!(
        !serialized.contains(
            &home
                .path()
                .join(".local/state/tak/arti/state")
                .display()
                .to_string()
        ),
        "serialized config should ignore HOME when XDG_STATE_HOME is present: {serialized}"
    );
}
