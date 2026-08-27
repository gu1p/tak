use std::fs;

use tak_core::model::RemoteTransportKind;
use tak_exec::{PlacementMode, RunOptions, run_tasks};

use crate::support::{EnvGuard, RetryableTorDaemon, env_lock};

#[tokio::test]
async fn tor_policy_falls_back_to_local_when_daemon_has_no_eligible_peer() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    fs::create_dir_all(&workspace_root).expect("workspace");
    let _daemon = RetryableTorDaemon::spawn_non_retryable(temp.path(), &mut env).await;
    let (spec, label) = super::support::policy_workspace(&workspace_root, RemoteTransportKind::Tor);

    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("policy fallback should run locally");
    let result = summary.results.get(&label).expect("summary result");
    assert_eq!(result.placement_mode, PlacementMode::Local);
    assert_eq!(
        fs::read_to_string(workspace_root.join("out/policy.txt")).expect("local output"),
        "local-fallback\n"
    );
}
