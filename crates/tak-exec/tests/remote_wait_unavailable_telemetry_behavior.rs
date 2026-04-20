#![allow(clippy::await_holding_lock)]

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, run_tasks};

use crate::support;

use support::remote_progress_wait::success_result;
use support::{
    CollectingStatusObserver, EnvGuard, EventPollPlan, RemoteInventoryRecord, ScriptedEventsServer,
    env_lock, remote_builder_spec, remote_task_spec, shell_step, write_remote_inventory,
};

#[tokio::test]
async fn wait_heartbeat_reports_unavailable_telemetry_without_failing_the_run() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());
    env.set("TAK_TEST_REMOTE_WAIT_HEARTBEAT_MS", "1500");

    let server = ScriptedEventsServer::spawn_with_status(
        "builder-heartbeat-missing-status",
        vec![EventPollPlan {
            delay: Duration::from_secs(2),
            events: Vec::new(),
            done: true,
        }],
        0,
        success_result("builder-heartbeat-missing-status"),
        false,
    );
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            "builder-heartbeat-missing-status",
            &server.base_url,
            "secret",
            "direct",
        )],
    );

    let observer = Arc::new(CollectingStatusObserver::default());
    let (spec, label) = remote_task_spec(
        &workspace_root,
        "delayed_terminal_events",
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
    .expect("remote wait run should succeed");

    assert!(summary.results.get(&label).expect("summary result").success);
    let statuses = observer.snapshot();
    assert!(statuses.iter().any(|event| {
        event
            .message
            .contains("remote task still running on builder-heartbeat-missing-status")
            && event.message.contains("node telemetry unavailable")
    }));
}
