use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, run_tasks};

use crate::support::{
    EnvGuard, RetryableTorDaemon, env_lock, remote_builder_spec, remote_task_spec, shell_step,
};

#[tokio::test]
async fn tor_submit_non_200_excludes_daemon_selected_worker_before_replacement() {
    let _lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");
    let daemon = RetryableTorDaemon::spawn_submit_failover(temp.path(), &mut env).await;
    let (spec, label) = remote_task_spec(
        &workspace,
        "tor-submit-failover",
        vec![shell_step("true")],
        remote_builder_spec(RemoteTransportKind::Tor),
    );

    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("Tor submit failover succeeds");

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

#[tokio::test]
async fn tor_submit_transport_error_excludes_daemon_selected_worker_before_replacement() {
    let _lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");
    let daemon = RetryableTorDaemon::spawn_submit_transport_failover(temp.path(), &mut env).await;
    let (spec, label) = remote_task_spec(
        &workspace,
        "tor-submit-transport-failover",
        vec![shell_step("true")],
        remote_builder_spec(RemoteTransportKind::Tor),
    );

    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("Tor submit transport failover succeeds");

    assert!(summary.results[&label].success);
    assert_eq!(
        summary.results[&label].remote_node_id.as_deref(),
        Some("builder-b")
    );
    assert_eq!(
        daemon.placement_exclusions().await,
        vec![Vec::<String>::new(), vec!["builder-a".to_string()]]
    );
}

#[tokio::test]
async fn tor_submit_exhaustion_names_workers_without_daemon_placeholder() {
    let _lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    std::fs::create_dir_all(&workspace).expect("workspace");
    let _daemon = RetryableTorDaemon::spawn_submit_exhaustion(temp.path(), &mut env).await;
    let (spec, label) = remote_task_spec(
        &workspace,
        "tor-submit-exhaustion",
        vec![shell_step("true")],
        remote_builder_spec(RemoteTransportKind::Tor),
    );

    let error = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect_err("all Tor submit candidates should fail");
    let message = format!("{error:#}");

    assert!(message.contains("builder-a"), "{message}");
    assert!(message.contains("builder-b"), "{message}");
    assert!(!message.contains("__takd_daemon_tor__"), "{message}");
}
