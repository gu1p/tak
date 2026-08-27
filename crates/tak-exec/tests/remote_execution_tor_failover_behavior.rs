#![allow(clippy::await_holding_lock)]

use std::sync::Arc;

use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, TaskStatusPhase, run_tasks};

use crate::support::{
    CollectingStatusObserver, EnvGuard, RetryableTorDaemon, env_lock, remote_builder_spec,
    remote_task_spec, shell_step,
};

mod submit_failure;

#[tokio::test]
async fn local_takd_tor_failover_reports_replacement_connection_and_excludes_failed_worker() {
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
    let observer = Arc::new(CollectingStatusObserver::default());

    let summary = run_tasks(
        &spec,
        std::slice::from_ref(&label),
        &RunOptions {
            output_observer: Some(observer.clone()),
            ..RunOptions::default()
        },
    )
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

    let statuses = observer.snapshot();
    let failover = statuses
        .iter()
        .position(|event| {
            event
                .message
                .contains("retrying on another eligible worker")
        })
        .expect("failover status");
    let replacement = &statuses[failover + 1..];
    assert_eq!(replacement[0].phase, TaskStatusPhase::RemoteProbe);
    assert_eq!(replacement[0].message, "connecting to local takd daemon");
    assert_eq!(
        replacement[1].message,
        "local takd: connecting to a replacement worker over Tor (excluding failed worker builder-a)"
    );
    assert!(replacement[2].message.starts_with("upload ["));
}
