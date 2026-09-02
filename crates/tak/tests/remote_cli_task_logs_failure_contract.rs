#![cfg(unix)]

use std::process::Command;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use prost::Message;
use tak_proto::{PollTaskEventsResponse, RemoteEvent};

use crate::support;
use support::remote_daemon_v2::FakeRemoteDaemon;

#[test]
fn remote_task_logs_preserves_terminal_failure_message_via_daemon_read() {
    assert_terminal(
        RemoteEvent {
            seq: 1,
            kind: "TASK_FAILED".into(),
            timestamp_ms: 12,
            success: Some(false),
            exit_code: Some(1),
            message: Some("worker exited before returning a result".into()),
            chunk: None,
            chunk_bytes: Vec::new(),
            queue_position: None,
        },
        "worker exited before returning a result\n",
    );
}

#[test]
fn remote_task_logs_preserves_terminal_exit_code_via_daemon_read() {
    assert_terminal(
        RemoteEvent {
            seq: 1,
            kind: "TASK_FAILED".into(),
            timestamp_ms: 12,
            success: Some(false),
            exit_code: Some(137),
            message: None,
            chunk: None,
            chunk_bytes: Vec::new(),
            queue_position: None,
        },
        "remote task failed with exit code 137\n",
    );
}

fn assert_terminal(event: RemoteEvent, expected_stderr: &str) {
    let root = tempfile::tempdir().expect("temp root");
    let body = PollTaskEventsResponse {
        events: vec![event],
        done: true,
    }
    .encode_to_vec();
    let daemon = FakeRemoteDaemon::spawn(
        root.path(),
        vec![serde_json::json!({
            "type": "RemoteRead", "node_id": "builder-a", "http_status": 200,
            "body_base64": STANDARD.encode(body)
        })],
    );
    let output = Command::new(support::tak_bin())
        .args([
            "remote",
            "task",
            "logs",
            "--node",
            "builder-a",
            "task-run-failed",
        ])
        .env("TAKD_SOCKET", daemon.socket())
        .output()
        .expect("run task logs");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(String::from_utf8_lossy(&output.stderr), expected_stderr);
    assert_eq!(daemon.finish()[0]["operation"]["type"], "ReadRemote");
}
