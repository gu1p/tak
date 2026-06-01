#![allow(clippy::await_holding_lock)]

use std::fs;
use std::sync::Arc;

use tak_core::model::RemoteTransportKind;
use tak_exec::{RunOptions, TaskStatusPhase, run_tasks};

use crate::support;

use support::{
    CollectingStatusObserver, EnvGuard, RemoteInventoryRecord, RunningTakdServer, env_lock,
    remote_builder_spec, remote_task_spec, shell_step, write_remote_inventory,
};

#[tokio::test]
async fn remote_status_reports_staged_upload_size_in_megabytes() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    fs::write(workspace_root.join("payload.bin"), vec![42_u8; 1_250_000]).expect("write payload");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());
    env.set(
        "TAKD_REMOTE_EXEC_ROOT",
        temp.path().join("remote-exec").display().to_string(),
    );

    let server = RunningTakdServer::spawn("builder-upload", "direct", temp.path()).await;
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            &server.node_id,
            &server.base_url,
            &server.bearer_token,
            "direct",
        )],
    );

    let observer = Arc::new(CollectingStatusObserver::default());
    let (spec, label) = remote_task_spec(
        &workspace_root,
        "remote_upload_size",
        vec![shell_step("test -s payload.bin")],
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
    .expect("remote run should succeed");

    assert!(summary.results.get(&label).expect("summary result").success);
    let statuses = observer.snapshot();
    assert!(statuses.iter().any(|event| {
        event.phase == TaskStatusPhase::RemoteStageWorkspace
            && event
                .message
                .starts_with("staged remote workspace (1 files, ")
            && event.message.ends_with(" MB upload)")
    }));
    assert!(statuses.iter().any(|event| {
        event.phase == TaskStatusPhase::RemoteSubmit
            && event.message.starts_with("upload [")
            && event.message.contains(" MB to remote node builder-upload")
    }));
}
