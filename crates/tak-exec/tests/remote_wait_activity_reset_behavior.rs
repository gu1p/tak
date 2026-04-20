#![allow(clippy::await_holding_lock)]

use std::fs;
use std::sync::Arc;

use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, run_tasks};

use crate::support;
#[path = "support/task_started_then_idle_plans.rs"]
mod task_started_then_idle_plans;

use support::remote_progress_wait::success_result;
use support::{
    CollectingStatusObserver, EnvGuard, RemoteInventoryRecord, ScriptedEventsServer, env_lock,
    remote_builder_spec, remote_task_spec, shell_step, write_remote_inventory,
};
use task_started_then_idle_plans::task_started_then_idle_plans;

#[tokio::test]
async fn task_started_event_resets_remote_inactivity_before_warning_is_due() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());
    env.set("TAK_TEST_REMOTE_WAIT_HEARTBEAT_MS", "1500");

    let server = ScriptedEventsServer::spawn(
        "builder-task-started",
        task_started_then_idle_plans(),
        8,
        success_result("builder-task-started"),
    );
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            "builder-task-started",
            &server.base_url,
            "secret",
            "direct",
        )],
    );

    let observer = Arc::new(CollectingStatusObserver::default());
    let (spec, label) = remote_task_spec(
        &workspace_root,
        "task_started_resets_wait",
        vec![shell_step("true")],
        remote_builder_spec(RemoteTransportKind::Direct),
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
    .expect("task-started run should succeed");

    assert!(summary.results.get(&label).expect("summary result").success);
    let statuses = observer.snapshot();
    assert!(
        !statuses.iter().any(|event| {
            event
                .message
                .contains("remote task still running on builder-task-started")
        }),
        "statuses:\n{statuses:#?}"
    );
}
