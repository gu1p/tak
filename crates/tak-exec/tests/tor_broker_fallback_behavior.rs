#![allow(clippy::await_holding_lock)]

use std::fs;

use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, run_tasks};

use crate::support::{
    EnvGuard, RemoteInventoryRecord, env_lock, remote_builder_spec, remote_task_spec, shell_step,
    write_remote_inventory,
};

#[tokio::test]
async fn tor_remote_execution_requires_default_local_takd_serve() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("workspace");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());
    env.set(
        "XDG_RUNTIME_DIR",
        temp.path().join("run").display().to_string(),
    );
    env.set(
        "TAKD_REMOTE_EXEC_ROOT",
        temp.path().join("remote-exec").display().to_string(),
    );

    env.set("TAK_TEST_TOR_ONION_DIAL_ADDR", "127.0.0.1:9");
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            "builder-no-daemon",
            "http://builder-no-daemon.onion",
            "secret",
            "tor",
        )],
    );

    let (spec, label) = remote_task_spec(
        &workspace_root,
        "remote_tor_default_broker_required",
        vec![shell_step("touch should-not-run")],
        remote_builder_spec(RemoteTransportKind::Tor),
    );
    let err = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect_err("tor run must require local takd serve");
    let rendered = format!("{err:#}");

    assert!(
        rendered.contains("Tor remote execution requires local takd serve"),
        "missing daemon diagnostic:\n{rendered}"
    );
    assert!(
        !workspace_root.join("should-not-run").exists(),
        "tor execution should fail before any in-process Tor dial"
    );
}
