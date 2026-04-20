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
async fn emits_wait_heartbeat_while_first_events_request_is_still_blocked() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());
    env.set("TAK_TEST_REMOTE_WAIT_HEARTBEAT_MS", "1500");

    let server = ScriptedEventsServer::spawn(
        "builder-heartbeat",
        vec![EventPollPlan {
            delay: Duration::from_secs(2),
            events: Vec::new(),
            done: true,
        }],
        0,
        success_result("builder-heartbeat"),
    );
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            "builder-heartbeat",
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
            .contains("remote task still running on builder-heartbeat")
            && event.message.contains("no new output for 1s")
            && event.message.contains("jobs=1")
            && event.message.contains("cpu=12.5%/8c")
            && event.message.contains("ram=2.0KiB/8.0KiB")
    }));
    assert!(
        !statuses
            .iter()
            .any(|event| event.message.contains("remote activity"))
    );
}
