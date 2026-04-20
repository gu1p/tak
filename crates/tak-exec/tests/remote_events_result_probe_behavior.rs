#![allow(clippy::await_holding_lock)]

use std::fs;
use std::sync::atomic::Ordering;

use tak_core::model::RemoteTransportKind;
use tak_exec::{OutputStream, RemoteLogChunk, RunOptions, run_tasks};

use crate::support;

use support::{
    EnvGuard, NonTerminalEventsServer, RemoteInventoryRecord, env_lock, remote_builder_spec,
    remote_task_spec, shell_step, write_remote_inventory,
};

#[tokio::test]
async fn falls_back_to_result_probe_when_terminal_event_never_arrives() {
    let _env_lock = env_lock();
    let mut env = EnvGuard::default();
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace_root = temp.path().join("workspace");
    let config_root = temp.path().join("config");
    fs::create_dir_all(&workspace_root).expect("create workspace");
    env.set("XDG_CONFIG_HOME", config_root.display().to_string());

    let server = NonTerminalEventsServer::spawn();
    write_remote_inventory(
        &config_root,
        &[RemoteInventoryRecord::builder(
            "builder-non-terminal",
            &server.base_url,
            "secret",
            "direct",
        )],
    );

    let (spec, label) = remote_task_spec(
        &workspace_root,
        "non_terminal_events",
        vec![shell_step("true")],
        remote_builder_spec(RemoteTransportKind::Direct),
    );
    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("remote result probe run should succeed");
    let result = summary.results.get(&label).expect("summary result");

    assert!(result.success);
    assert_eq!(
        result.remote_logs,
        vec![RemoteLogChunk {
            seq: 1,
            stream: OutputStream::Stdout,
            bytes: b"pending\n".to_vec(),
        }]
    );
    assert!(server.events_calls.load(Ordering::SeqCst) >= 3);
}
