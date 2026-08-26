#![allow(clippy::await_holding_lock)]

use tak_exec::{RunOptions, run_tasks};

use crate::support::{
    EnvGuard, RecordingEvents, RecordingRemoteServer, RemoteInventoryRecord, env_lock,
    fused_remote_cascade_spec, remote_task_spec, shell_step, write_remote_inventory,
};

#[tokio::test]
async fn fused_cascade_fails_over_as_one_logical_attempt() {
    let _lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace");
    env.set("XDG_CONFIG_HOME", config.display().to_string());
    let a =
        RecordingRemoteServer::spawn_infrastructure_137("builder-a", RecordingEvents::default());
    let b = RecordingRemoteServer::spawn_success("builder-b", RecordingEvents::default());
    write_remote_inventory(
        &config,
        &[
            RemoteInventoryRecord::builder("builder-a", &a.base_url, "secret", "direct"),
            RemoteInventoryRecord::builder("builder-b", &b.base_url, "secret", "direct"),
        ],
    );
    let (mut spec, _) = remote_task_spec(
        &workspace,
        "seed",
        vec![shell_step("true")],
        crate::support::remote_builder_spec(tak_core::model::RemoteTransportKind::Direct),
    );
    let target = fused_remote_cascade_spec(&mut spec);

    let summary = run_tasks(&spec, std::slice::from_ref(&target), &RunOptions::default())
        .await
        .expect("fused failover");

    assert!(summary.results[&target].success);
    assert_eq!(summary.results[&target].attempts, 1);
    assert_eq!(
        summary.results[&target].remote_node_id.as_deref(),
        Some("builder-b")
    );
}
