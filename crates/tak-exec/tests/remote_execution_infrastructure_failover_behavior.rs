#![allow(clippy::await_holding_lock)]
use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, run_tasks};

use crate::support::{
    LockedEnvGuard, RecordingEvents, RecordingRemoteServer, RemoteInventoryRecord,
    remote_builder_spec, remote_task_spec, shell_step, write_remote_inventory,
};

#[tokio::test]
async fn exit_137_fails_over_without_consuming_authored_attempts() {
    let (_env, workspace, _temp, first_events, _first, _second_events, _second) = setup(false);
    let (spec, label) = remote_task_spec(
        &workspace,
        "failover",
        vec![shell_step("true")],
        remote_builder_spec(RemoteTransportKind::Direct),
    );
    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("infrastructure failover succeeds");
    let result = summary.results.get(&label).expect("result");
    assert!(result.success);
    assert_eq!(result.attempts, 1);
    assert_eq!(result.remote_node_id.as_deref(), Some("builder-b"));
    assert_eq!(submit_count(&first_events), 1);
}

#[tokio::test]
async fn ordinary_exit_1_is_terminal_on_the_first_worker() {
    let (_env, workspace, _temp, first_events, _first, second, _second) = setup(true);
    let (spec, label) = remote_task_spec(
        &workspace,
        "task-failure",
        vec![shell_step("false")],
        remote_builder_spec(RemoteTransportKind::Direct),
    );
    let error = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect_err("ordinary task failure is terminal");
    assert!(format!("{error:#}").contains("builder-a failed with exit 1"));
    assert_eq!(submit_count(&first_events), 1);
    assert_eq!(submit_count(&second), 0);
}

fn setup(
    task_failure: bool,
) -> (
    LockedEnvGuard,
    std::path::PathBuf,
    tempfile::TempDir,
    RecordingEvents,
    RecordingRemoteServer,
    RecordingEvents,
    RecordingRemoteServer,
) {
    let mut env = LockedEnvGuard::acquire();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace");
    env.set("XDG_CONFIG_HOME", config.display().to_string());
    let first_events = RecordingEvents::default();
    let second_events = RecordingEvents::default();
    let first = if task_failure {
        RecordingRemoteServer::spawn_task_exit_1("builder-a", first_events.clone())
    } else {
        RecordingRemoteServer::spawn_infrastructure_137("builder-a", first_events.clone())
    };
    let second = RecordingRemoteServer::spawn_success("builder-b", second_events.clone());
    write_remote_inventory(
        &config,
        &[
            RemoteInventoryRecord::builder("builder-a", &first.base_url, "secret", "direct"),
            RemoteInventoryRecord::builder("builder-b", &second.base_url, "secret", "direct"),
        ],
    );
    (
        env,
        workspace,
        temp,
        first_events,
        first,
        second_events,
        second,
    )
}

fn submit_count(events: &RecordingEvents) -> usize {
    events
        .snapshot()
        .iter()
        .filter(|event| event.as_str() == "remote_submit")
        .count()
}
