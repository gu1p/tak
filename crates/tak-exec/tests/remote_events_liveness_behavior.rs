#![allow(clippy::await_holding_lock)]

use std::fs;

use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, run_tasks};

use crate::support;

use support::{
    EnvGuard, RemoteInventoryRecord, RunningTakdServer, env_lock, remote_builder_spec,
    remote_task_spec, shell_step, write_remote_inventory,
};

#[tokio::test]
async fn healthy_event_polls_outlive_legacy_total_wait_budget() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());
    env.set(
        "TAKD_REMOTE_EXEC_ROOT",
        temp.path().join("remote-exec").display().to_string(),
    );
    env.set("TAK_REMOTE_EVENTS_MAX_WAIT_SECS", "1");

    let server = RunningTakdServer::spawn("builder-a", "direct", temp.path()).await;
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            &server.node_id,
            &server.base_url,
            &server.bearer_token,
            "direct",
        )],
    );

    let (spec, label) = remote_task_spec(
        &workspace_root,
        "slow_events",
        vec![shell_step("sleep 2")],
        remote_builder_spec(RemoteTransportKind::Direct),
    );
    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("remote run should succeed");

    assert!(summary.results.get(&label).expect("summary result").success);
}
