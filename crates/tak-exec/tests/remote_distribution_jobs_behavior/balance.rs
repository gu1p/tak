use super::support::{node_count, remote_workspace, remote_workspace_with_selection};
use crate::support::{
    EnvGuard, RemoteInventoryRecord, RunningTakdServer, env_lock, write_remote_inventory,
};
use tak_core::model::{RemoteSelectionSpec, TaskLabel};
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

#[tokio::test]
async fn round_robin_remote_jobs_balance_across_equal_nodes_in_one_run() {
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
    let spec =
        remote_workspace_with_selection(&workspace, &labels, RemoteSelectionSpec::RoundRobin);
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
