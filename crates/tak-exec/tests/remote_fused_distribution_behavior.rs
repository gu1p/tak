#![allow(clippy::await_holding_lock)]

use std::collections::BTreeSet;

use tak_core::model::{RemoteSelectionSpec, TaskExecutionSpec};
use tak_exec::{RunOptions, run_tasks};

use crate::support::{
    EnvGuard, RemoteInventoryRecord, RunningTakdServer, env_lock, fused_remote_cascade_spec,
    remote_task_spec, shell_step, write_remote_inventory,
};

#[tokio::test]
async fn shuffled_fused_cascade_stays_on_one_remote_node() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace");
    env.set("XDG_CONFIG_HOME", config.display().to_string());
    env.set(
        "TAKD_REMOTE_EXEC_ROOT",
        temp.path().join("remote-exec").display().to_string(),
    );
    let a = RunningTakdServer::spawn("builder-a", "direct", temp.path()).await;
    let b = RunningTakdServer::spawn("builder-b", "direct", temp.path()).await;
    write_remote_inventory(
        &config,
        &[
            RemoteInventoryRecord::builder(&a.node_id, &a.base_url, &a.bearer_token, "direct"),
            RemoteInventoryRecord::builder(&b.node_id, &b.base_url, &b.bearer_token, "direct"),
        ],
    );

    let (mut spec, _) = remote_task_spec(
        &workspace,
        "seed",
        vec![shell_step("true")],
        crate::support::remote_builder_spec(tak_core::model::RemoteTransportKind::Direct),
    );
    let target = fused_remote_cascade_spec(&mut spec);
    for task in spec.tasks.values_mut() {
        if let TaskExecutionSpec::RemoteOnly(remote) = &mut task.execution {
            remote.selection = RemoteSelectionSpec::Shuffle;
        }
    }

    let summary = run_tasks(&spec, &[target], &RunOptions::default())
        .await
        .expect("fused remote run");
    assert_eq!(unique_remote_nodes(&summary).len(), 1);
    assert_eq!(unique_task_runs(&summary).len(), 1);
}

fn unique_remote_nodes(summary: &tak_exec::RunSummary) -> BTreeSet<String> {
    summary
        .results
        .values()
        .filter_map(|result| result.remote_node_id.clone())
        .collect()
}

fn unique_task_runs(summary: &tak_exec::RunSummary) -> BTreeSet<String> {
    summary
        .results
        .values()
        .map(|result| result.task_run_id.clone())
        .collect()
}
