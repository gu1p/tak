#![allow(clippy::await_holding_lock)]
mod support;
mod tor;

use crate::support::{EnvGuard, RemoteInventoryRecord, env_lock, write_remote_inventory};
use std::fs;
use tak_core::model::RemoteTransportKind;
use tak_exec::{PlacementMode, RunOptions, run_tasks};
#[tokio::test]
async fn execution_policy_falls_back_to_local_before_remote_task_start() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("workspace");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            "unreachable",
            "not a valid endpoint",
            "secret",
            "direct",
        )],
    );
    let (spec, label) = support::policy_workspace(&workspace_root, RemoteTransportKind::Direct);
    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("policy fallback should run locally");
    let result = summary.results.get(&label).expect("summary result");
    assert_eq!(result.placement_mode, PlacementMode::Local);
    assert_eq!(
        fs::read_to_string(workspace_root.join("out/policy.txt")).expect("local output"),
        "local-fallback\n"
    );
}
