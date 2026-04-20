#![allow(clippy::await_holding_lock)]

use std::fs;

use tak_core::model::RemoteTransportKind;
use tak_exec::{PlacementMode, RunOptions, run_tasks};

use crate::support;

use support::{
    EnvGuard, RemoteInventoryRecord, RunningTakdServer, env_lock, remote_builder_spec,
    remote_task_spec_with_outputs, shell_step, workspace_output_path, write_remote_inventory,
};

#[tokio::test]
async fn remote_execution_syncs_outputs_with_reserved_query_characters_in_path() {
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

    let output_paths = ["reports/run#1.txt", "reports/a+b.txt", "reports/a%2Fb.txt"];
    let (spec, label) = remote_task_spec_with_outputs(
        &workspace_root,
        "remote_sync_reserved_paths",
        vec![shell_step(
            "mkdir -p reports && cp src/input.txt 'reports/run#1.txt' && cp src/input.txt 'reports/a+b.txt' && cp src/input.txt 'reports/a%2Fb.txt'",
        )],
        remote_builder_spec(RemoteTransportKind::Direct),
        output_paths
            .iter()
            .map(|path| workspace_output_path(path))
            .collect(),
    );

    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("remote run should succeed");
    let result = summary.results.get(&label).expect("summary result");

    assert_eq!(result.placement_mode, PlacementMode::Remote);
    assert_eq!(result.remote_node_id.as_deref(), Some("builder-a"));
    assert_eq!(result.remote_transport_kind.as_deref(), Some("direct"));
    assert_eq!(result.synced_outputs.len(), output_paths.len());
    for path in output_paths {
        assert_eq!(
            fs::read_to_string(workspace_root.join(path)).expect("synced output"),
            "hello remote\n"
        );
    }
}
