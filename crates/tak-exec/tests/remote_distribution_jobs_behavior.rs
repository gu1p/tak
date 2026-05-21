#![allow(clippy::await_holding_lock)]

use crate::support::{
    EnvGuard, RemoteInventoryRecord, RunningTakdServer, env_lock, remote_builder_spec, shell_step,
    write_remote_inventory,
};
use std::collections::{BTreeMap, HashMap};
use tak_core::model::{ContainerRuntimeSourceSpec::Image, RemoteRuntimeSpec::Containerized};
use tak_core::model::{
    CurrentStateSpec, LimiterKey, QueueDef, RemoteSelectionSpec, RemoteTransportKind, ResolvedTask,
    RetryDef, TaskExecutionSpec, TaskLabel, WorkspaceSpec,
};
use tak_exec::{RunOptions, run_tasks};
#[tokio::test]
async fn shuffled_remote_jobs_balance_across_equal_nodes() {
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
    let labels = (0..6)
        .map(|index| TaskLabel {
            package: "//".into(),
            name: format!("t{index}"),
        })
        .collect::<Vec<_>>();
    let spec = remote_workspace(&workspace, &labels);
    let options = RunOptions {
        jobs: 6,
        ..RunOptions::default()
    };
    let summary = run_tasks(&spec, &labels, &options)
        .await
        .expect("parallel remote run");
    assert_eq!(node_count(&summary, "builder-a"), 3);
    assert_eq!(node_count(&summary, "builder-b"), 3);
}
fn remote_workspace(root: &std::path::Path, labels: &[TaskLabel]) -> WorkspaceSpec {
    let tasks = labels
        .iter()
        .map(|label| (label.clone(), remote_task(label)))
        .collect();
    WorkspaceSpec {
        project_id: "remote-jobs-test".into(),
        root: root.to_path_buf(),
        tasks,
        sessions: BTreeMap::new(),
        limiters: HashMap::<LimiterKey, _>::new(),
        queues: HashMap::<LimiterKey, QueueDef>::new(),
    }
}
fn remote_task(label: &TaskLabel) -> ResolvedTask {
    let mut remote = remote_builder_spec(RemoteTransportKind::Direct);
    remote.selection = RemoteSelectionSpec::Shuffle;
    ResolvedTask {
        label: label.clone(),
        doc: String::new(),
        deps: Vec::new(),
        steps: vec![shell_step("true")],
        needs: Vec::new(),
        queue: None,
        retry: RetryDef::default(),
        timeout_s: None,
        context: CurrentStateSpec::default(),
        outputs: Vec::new(),
        container_runtime: Some(Containerized {
            source: Image {
                image: "alpine:3.20".into(),
            },
        }),
        execution: TaskExecutionSpec::RemoteOnly(remote),
        session: None,
        cascade_execution: false,
        tags: Vec::new(),
    }
}
fn node_count(summary: &tak_exec::RunSummary, node_id: &str) -> usize {
    summary
        .results
        .values()
        .filter(|result| result.remote_node_id.as_deref() == Some(node_id))
        .count()
}
