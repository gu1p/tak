#![cfg(unix)]

use std::process::Command;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use prost::Message;
use tak_proto::{PollTaskEventsResponse, RemoteEvent};

use crate::support;
use support::remote_daemon_v2::FakeRemoteDaemon;

#[test]
fn remote_task_logs_preserves_streams_via_daemon_v2_read() {
    let root = tempfile::tempdir().expect("temp root");
    let event = |seq, kind: &str, bytes: &[u8]| RemoteEvent {
        seq,
        kind: kind.into(),
        timestamp_ms: 10,
        success: None,
        exit_code: None,
        message: None,
        chunk: None,
        chunk_bytes: bytes.to_vec(),
        queue_position: None,
    };
    let body = PollTaskEventsResponse {
        events: vec![
            event(1, "TASK_STDOUT_CHUNK", b"remote stdout\n"),
            event(2, "TASK_STDERR_CHUNK", b"remote stderr\n"),
        ],
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
            "task-run-remote-1",
        ])
        .env("TAKD_SOCKET", daemon.socket())
        .output()
        .expect("run remote task logs");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout), "remote stdout\n");
    assert_eq!(String::from_utf8_lossy(&output.stderr), "remote stderr\n");
    let requests = daemon.finish();
    assert_eq!(requests[0]["operation"]["type"], "ReadRemote");
    assert_eq!(
        requests[0]["operation"]["path"],
        "/v2/worker/tasks/task-run-remote-1/events?after_seq=0"
    );
}
