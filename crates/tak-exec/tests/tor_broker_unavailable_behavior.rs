#![allow(clippy::await_holding_lock)]

use std::fs;

use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, run_tasks};

use crate::support;

use support::{
    EnvGuard, RemoteInventoryRecord, env_lock, remote_builder_spec, remote_task_spec, shell_step,
    write_remote_inventory,
};

#[tokio::test]
async fn tor_remote_execution_reports_unavailable_local_broker() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("workspace");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());
    env.set(
        "TAKD_SOCKET",
        temp.path().join("missing-takd.sock").display().to_string(),
    );
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            "builder-broker-missing",
            "http://builder-broker-missing.onion",
            "secret",
            "tor",
        )],
    );

    let (spec, label) = remote_task_spec(
        &workspace_root,
        "remote_tor_missing_broker",
        vec![shell_step("echo should-not-run")],
        remote_builder_spec(RemoteTransportKind::Tor),
    );
    let err = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect_err("tor execution should require the local broker");
    let rendered = format!("{err:#}");

    assert!(
        rendered.contains("Tor remote execution requires local takd serve"),
        "missing local daemon diagnostic:\n{rendered}"
    );
}
