#![allow(clippy::await_holding_lock)]

use std::fs;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use tak_core::model::RemoteTransportKind;
use tak_exec::{
    OutputStream, RemoteLogChunk, RunOptions, TaskOutputChunk, TaskOutputObserver, run_tasks,
};

mod support;

use support::{
    DelayedEventsServer, EnvGuard, RemoteInventoryRecord, env_lock, remote_builder_spec,
    remote_task_spec, shell_step, write_remote_inventory,
};

#[derive(Default)]
struct CollectingObserver {
    chunks: Mutex<Vec<TaskOutputChunk>>,
}

impl CollectingObserver {
    fn snapshot(&self) -> Vec<TaskOutputChunk> {
        self.chunks.lock().expect("observer lock").clone()
    }
}

impl TaskOutputObserver for CollectingObserver {
    fn observe_output(&self, chunk: TaskOutputChunk) -> anyhow::Result<()> {
        self.chunks.lock().expect("observer lock").push(chunk);
        Ok(())
    }
}

#[tokio::test]
async fn waits_for_terminal_protobuf_events_without_duplicate_logs() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());
    env.set("TAK_REMOTE_EVENTS_MAX_WAIT_SECS", "5");

    let server = DelayedEventsServer::spawn();
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            "builder-delayed",
            &server.base_url,
            "secret",
            "direct",
        )],
    );

    let (spec, label) = remote_task_spec(
        &workspace_root,
        "delayed_events",
        vec![shell_step("true")],
        remote_builder_spec(RemoteTransportKind::Direct),
    );
    let observer = Arc::new(CollectingObserver::default());
    let summary = run_tasks(
        &spec,
        std::slice::from_ref(&label),
        &RunOptions {
            output_observer: Some(observer.clone()),
            ..RunOptions::default()
        },
    )
    .await
    .expect("remote polling run should succeed");
    let result = summary.results.get(&label).expect("summary result");

    assert_eq!(
        result.remote_logs,
        vec![RemoteLogChunk {
            seq: 1,
            stream: OutputStream::Stdout,
            bytes: b"pending\n".to_vec(),
        }]
    );
    assert_eq!(
        observer.snapshot(),
        vec![TaskOutputChunk {
            task_label: label.clone(),
            attempt: 1,
            stream: OutputStream::Stdout,
            bytes: b"pending\n".to_vec(),
        }]
    );
    assert!(server.events_calls.load(Ordering::SeqCst) >= 3);
}
