#![allow(clippy::await_holding_lock)]

use std::fs;

use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, run_tasks};

use crate::support;
use support::{
    EnvGuard, LocalTorBroker, RemoteInventoryRecord, env_lock, remote_builder_spec,
    remote_task_spec, shell_step, write_remote_inventory,
};

#[tokio::test]
async fn tor_remote_execution_reports_remote_failure_when_broker_is_reachable() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("workspace");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());
    env.set("TAK_TEST_TOR_PROBE_TIMEOUT_MS", "200");
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            "builder-remote-down",
            "http://builder-remote-down.onion",
            "secret",
            "tor",
        )],
    );
    let broker = LocalTorBroker::spawn(temp.path(), "127.0.0.1:9", &mut env).await;

    let (spec, label) = remote_task_spec(
        &workspace_root,
        "remote_tor_unreachable_remote",
        vec![shell_step("echo should-not-run")],
        remote_builder_spec(RemoteTransportKind::Tor),
    );
    let err = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect_err("remote failure should fail preflight");
    let rendered = format!("{err:#}");

    assert!(
        rendered.contains("timed out while contacting remote node builder-remote-down")
            || rendered.contains("remote node builder-remote-down unavailable")
            || rendered.contains("remote node builder-remote-down")
                && rendered.contains("node info request timed out"),
        "missing remote failure diagnostic:\n{rendered}"
    );
    assert!(
        !rendered.contains("local takd daemon unavailable"),
        "remote failure should not be reported as local daemon failure:\n{rendered}"
    );
    drop(broker);
}
