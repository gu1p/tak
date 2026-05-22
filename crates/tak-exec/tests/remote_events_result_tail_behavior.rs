#![allow(clippy::await_holding_lock)]

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use tak_core::model::RemoteTransportKind;
use tak_exec::{OutputStream, RemoteLogChunk, RunOptions, TaskOutputChunk, run_tasks};
use tak_proto::RemoteEvent;

use crate::support;

use support::remote_progress_wait::success_result;
use support::{
    CollectingObserver, EnvGuard, EventPollPlan, RemoteInventoryRecord, ScriptedEventsServer,
    env_lock, remote_builder_spec, remote_task_spec, shell_step, write_remote_inventory,
};

#[tokio::test]
async fn recovers_missing_stdout_from_remote_result_tail() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());

    let mut result = success_result("builder-tail");
    result.stdout_tail = Some("image\n".into());
    let server = ScriptedEventsServer::spawn(
        "builder-tail",
        vec![EventPollPlan {
            delay: Duration::ZERO,
            events: vec![RemoteEvent {
                seq: 1,
                kind: "TASK_COMPLETED".into(),
                timestamp_ms: 1,
                success: Some(true),
                exit_code: Some(0),
                message: None,
                chunk: None,
                chunk_bytes: Vec::new(),
            }],
            done: true,
        }],
        0,
        result,
    );
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            "builder-tail",
            &server.base_url,
            "secret",
            "direct",
        )],
    );

    let observer = Arc::new(CollectingObserver::default());
    let (spec, label) = remote_task_spec(
        &workspace_root,
        "result_tail",
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
    .expect("remote run should recover result tail");
    let result = summary.results.get(&label).expect("summary result");

    assert!(result.success);
    assert_eq!(
        result.remote_logs,
        vec![RemoteLogChunk {
            seq: 1,
            stream: OutputStream::Stdout,
            bytes: b"image\n".to_vec(),
        }]
    );
    assert_eq!(
        observer.snapshot().as_slice(),
        &[TaskOutputChunk {
            task_run_id: result.task_run_id.clone(),
            task_label: label,
            attempt: 1,
            stream: OutputStream::Stdout,
            bytes: b"image\n".to_vec(),
        }]
    );
}
