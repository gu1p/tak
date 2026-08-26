#![allow(clippy::await_holding_lock)]

use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, run_tasks};

use crate::support::{
    EnvGuard, RecordingEvents, RecordingRemoteServer, RemoteInventoryRecord, env_lock,
    remote_builder_spec, remote_task_spec, shell_step, write_remote_inventory,
};

#[tokio::test]
async fn infrastructure_exhaustion_lists_each_distinct_worker_once() {
    let _lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    let config = temp.path().join("config");
    std::fs::create_dir_all(&workspace).expect("workspace");
    env.set("XDG_CONFIG_HOME", config.display().to_string());
    let a_events = RecordingEvents::default();
    let b_events = RecordingEvents::default();
    let a = RecordingRemoteServer::spawn_infrastructure_137("builder-a", a_events.clone());
    let b = RecordingRemoteServer::spawn_infrastructure_137("builder-b", b_events.clone());
    write_remote_inventory(
        &config,
        &[
            RemoteInventoryRecord::builder("builder-a", &a.base_url, "secret", "direct"),
            RemoteInventoryRecord::builder("builder-b", &b.base_url, "secret", "direct"),
        ],
    );
    let (spec, label) = remote_task_spec(
        &workspace,
        "exhaust",
        vec![shell_step("true")],
        remote_builder_spec(RemoteTransportKind::Direct),
    );

    let error = run_tasks(&spec, &[label], &RunOptions::default())
        .await
        .expect_err("all workers fail")
        .to_string();

    assert!(
        error.contains("builder-a") && error.contains("exit 137"),
        "{error}"
    );
    assert!(
        error.contains("builder-b") && error.contains("exit 137"),
        "{error}"
    );
    assert_eq!(submits(&a_events), 1);
    assert_eq!(submits(&b_events), 1);
}

fn submits(events: &RecordingEvents) -> usize {
    events
        .snapshot()
        .iter()
        .filter(|event| event.as_str() == "remote_submit")
        .count()
}
