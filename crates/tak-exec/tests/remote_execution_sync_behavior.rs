#![allow(clippy::await_holding_lock)]

use std::fs;

use tak_core::model::RemoteTransportKind;
use tak_exec::{PlacementMode, RunOptions, run_tasks, target_set_from_summary};

mod support;

use support::{
    EnvGuard, RemoteInventoryRecord, RunningTakdServer, env_lock, remote_builder_spec,
    remote_task_spec, shell_step, write_remote_inventory,
};

#[tokio::test]
async fn remote_execution_uses_real_takd_server_and_syncs_outputs() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(workspace_root.join("src")).expect("create workspace");
    fs::write(workspace_root.join("src/input.txt"), "hello remote\n").expect("write input");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());
    env.set(
        "TAKD_REMOTE_EXEC_ROOT",
        temp.path().join("remote-exec").display().to_string(),
    );

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
        "remote_sync",
        vec![shell_step("mkdir -p dist && cp src/input.txt dist/out.txt")],
        remote_builder_spec(RemoteTransportKind::Direct),
    );
    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("remote run should succeed");
    let result = summary.results.get(&label).expect("summary result");

    assert_eq!(result.placement_mode, PlacementMode::Remote);
    assert_eq!(result.remote_node_id.as_deref(), Some("builder-a"));
    assert_eq!(result.remote_transport_kind.as_deref(), Some("direct"));
    assert!(result.context_manifest_hash.is_some());
    assert_eq!(
        fs::read_to_string(workspace_root.join("dist/out.txt")).expect("synced output"),
        "hello remote\n"
    );
    assert_eq!(result.synced_outputs.len(), 1);
    assert!(result.remote_logs.is_empty());
    assert!(target_set_from_summary(&summary).contains(&label));
}
