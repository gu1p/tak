#![allow(clippy::await_holding_lock)]

use std::fs;
use std::sync::atomic::Ordering;

use tak_core::model::RemoteTransportKind;
use tak_exec::{RemoteLogChunk, RunOptions, run_tasks};

mod support;

use support::{
    DelayedEventsServer, EnvGuard, RemoteInventoryRecord, env_lock, remote_builder_spec,
    remote_task_spec, shell_step, write_remote_inventory,
};

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
    let summary = run_tasks(&spec, std::slice::from_ref(&label), &RunOptions::default())
        .await
        .expect("remote polling run should succeed");
    let result = summary.results.get(&label).expect("summary result");

    assert_eq!(
        result.remote_logs,
        vec![RemoteLogChunk {
            seq: 1,
            chunk: "pending\n".into()
        }]
    );
    assert!(server.events_calls.load(Ordering::SeqCst) >= 3);
}
