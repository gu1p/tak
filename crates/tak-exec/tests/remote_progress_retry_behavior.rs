#![allow(clippy::await_holding_lock)]

use std::fs;
use std::sync::Arc;

use tak_core::model::{BackoffDef, RemoteTransportKind, RetryDef};
use tak_exec::{RunOptions, TaskStatusPhase, run_tasks};

mod support;

use support::{
    CollectingStatusObserver, EnvGuard, RemoteInventoryRecord, RunningTakdServer, env_lock,
    remote_builder_spec, remote_task_spec_with_outputs, shell_step, workspace_output_path,
    write_remote_inventory,
};

#[tokio::test]
async fn remote_status_reports_retry_backoff_before_the_next_attempt() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());
    env.set(
        "TAKD_REMOTE_EXEC_ROOT",
        temp.path().join("remote-exec").display().to_string(),
    );

    let server = RunningTakdServer::spawn("builder-retry", "direct", temp.path()).await;
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            &server.node_id,
            &server.base_url,
            &server.bearer_token,
            "direct",
        )],
    );

    let retry_state = temp.path().join("retry-state");
    let script = format!(
        "if [ ! -f '{0}' ]; then touch '{0}'; exit 7; fi; echo retried > out.txt",
        retry_state.display()
    );
    let observer = Arc::new(CollectingStatusObserver::default());
    let (mut spec, label) = remote_task_spec_with_outputs(
        &workspace_root,
        "remote_retry",
        vec![shell_step(&script)],
        remote_builder_spec(RemoteTransportKind::Direct),
        vec![workspace_output_path("out.txt")],
    );
    spec.tasks.get_mut(&label).expect("task").retry = RetryDef {
        attempts: 2,
        on_exit: vec![7],
        backoff: BackoffDef::Fixed { seconds: 2.0 },
    };
    let summary = run_tasks(
        &spec,
        std::slice::from_ref(&label),
        &RunOptions {
            output_observer: Some(observer.clone()),
            ..RunOptions::default()
        },
    )
    .await
    .expect("retry should succeed");

    let statuses = observer.snapshot();
    assert!(summary.results.get(&label).expect("summary result").success);
    assert!(
        statuses
            .iter()
            .any(|event| event.phase == TaskStatusPhase::RetryWait
                && event.message.contains("retrying after failure in 2s"))
    );
    assert!(statuses.iter().any(|event| {
        event.attempt == 2
            && event.phase == TaskStatusPhase::RemoteSubmit
            && event
                .message
                .contains("submitting to remote node builder-retry")
    }));
    assert_eq!(
        fs::read_to_string(workspace_root.join("out.txt")).expect("retried output"),
        "retried\n"
    );
}
