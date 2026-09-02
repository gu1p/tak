#![cfg(unix)]

use std::process::Command;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use prost::Message;
use tak_proto::{ListTaskAttemptsResponse, TaskAttemptSummary};

use crate::support;
use support::remote_daemon_v2::FakeRemoteDaemon;

#[test]
fn remote_tasks_preserves_rendering_via_daemon_v2_read() {
    let root = tempfile::tempdir().expect("temp root");
    let body = ListTaskAttemptsResponse {
        attempts: vec![TaskAttemptSummary {
            task_run_id: "task-run-remote-1".into(),
            attempt: 1,
            task_label: "//apps/web:build".into(),
            node_id: "builder-a".into(),
            state: "completed".into(),
            created_at_ms: 10,
            finished_at_ms: Some(20),
            execution_label: Some("check.build".into()),
        }],
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
        .args(["remote", "tasks", "--node", "builder-a"])
        .env("TAKD_SOCKET", daemon.socket())
        .output()
        .expect("run remote tasks");

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Remote Tasks") && stdout.contains("task_label=check.build"));
    assert!(stdout.contains("task_run_id=task-run-remote-1") && stdout.contains("state=completed"));
    let requests = daemon.finish();
    assert_eq!(requests[0]["operation"]["type"], "ReadRemote");
    assert_eq!(
        requests[0]["operation"]["path"],
        "/v2/worker/tasks?state=all&limit=50"
    );
}
