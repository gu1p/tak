#![allow(clippy::await_holding_lock)]

use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, run_tasks};

use crate::support::{
    EnvGuard, RetryableTorDaemon, env_lock, remote_builder_spec, remote_task_spec, shell_step,
};

#[tokio::test]
async fn local_takd_tor_placement_excludes_the_failed_worker() {
    let _lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");
    let daemon = RetryableTorDaemon::spawn_failover(temp.path(), &mut env).await;
    let (spec, label) = remote_task_spec(
        &workspace,
        "tor-failover",
        vec![shell_step("true")],
        remote_builder_spec(RemoteTransportKind::Tor),
    );

    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("Tor failover succeeds");

    assert!(summary.results[&label].success);
    assert_eq!(summary.results[&label].attempts, 1);
    assert_eq!(
        summary.results[&label].remote_node_id.as_deref(),
        Some("builder-b")
    );
    assert_eq!(daemon.submit_attempts().await, vec![1, 1]);
    assert_eq!(
        daemon.placement_exclusions().await,
        vec![Vec::<String>::new(), vec!["builder-a".to_string()]]
    );
}
