#![allow(clippy::await_holding_lock)]

use std::fs;
use std::sync::Arc;

use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, TaskStatusPhase, run_tasks};

use crate::support;

use support::{
    CollectingStatusObserver, EnvGuard, RetryableTorDaemon, env_lock, remote_builder_spec,
    remote_task_spec, shell_step,
};

#[tokio::test]
async fn tor_setup_retries_retryable_upload_failure_on_next_public_attempt() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    fs::write(workspace_root.join("input.txt"), "retry me").expect("workspace input");
    let daemon = RetryableTorDaemon::spawn(temp.path(), &mut env).await;
    let observer = Arc::new(CollectingStatusObserver::default());

    let (spec, label) = remote_task_spec(
        &workspace_root,
        "remote_tor_retryable_setup",
        vec![shell_step("true")],
        remote_builder_spec(RemoteTransportKind::Tor),
    );
    let summary = run_tasks(
        &spec,
        std::slice::from_ref(&label),
        &RunOptions {
            output_observer: Some(observer.clone()),
            ..RunOptions::default()
        },
    )
    .await
    .expect("retryable setup failure should retry and succeed");

    let result = summary.results.get(&label).expect("task result");
    assert!(result.success);
    assert_eq!(result.attempts, 2);
    assert_eq!(result.remote_node_id.as_deref(), Some("builder-retry"));
    assert_eq!(daemon.submit_attempts().await, vec![2]);
    assert_eq!(daemon.distinct_upload_ids().await, 1);
    assert!(
        daemon.stream_offsets().await.contains(&8),
        "retry should resume from the committed offset"
    );
    assert!(observer.snapshot().iter().any(|event| {
        event.phase == TaskStatusPhase::RetryWait
            && event.attempt == 2
            && event.message.contains("retrying remote setup")
    }));
}

#[tokio::test]
async fn tor_setup_does_not_retry_non_retryable_daemon_placement_failure() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    let daemon = RetryableTorDaemon::spawn_non_retryable(temp.path(), &mut env).await;

    let (spec, label) = remote_task_spec(
        &workspace_root,
        "remote_tor_impossible",
        vec![shell_step("true")],
        remote_builder_spec(RemoteTransportKind::Tor),
    );
    let error = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect_err("non-retryable setup failure should fail");

    assert!(error.to_string().contains("retryable: no"));
    assert_eq!(daemon.peer_requests().await, 1);
    assert!(daemon.submit_attempts().await.is_empty());
}
