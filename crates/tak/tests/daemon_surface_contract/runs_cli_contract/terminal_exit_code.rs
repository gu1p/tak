use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn attach_returns_the_persisted_process_exit_code() {
    let response = json!({
        "protocol_version": 2, "type": "RunEvents", "request_id": "tak-runs-attach",
        "run_id": "run-1", "events": [{"seq": 1, "kind": "failed",
            "job_id": "job-0", "task_ids": ["//:exec"], "node_id": "local",
            "message": "job failed", "exit_code": 7}],
        "next_event": 1, "state": "failed", "terminal": true,
        "logs_expired": false, "exit_code": 7,
    });
    let output = execute(&["runs", "attach", "run-1"], response);
    assert_eq!(output.status.code(), Some(7));
    assert!(String::from_utf8_lossy(&output.stderr).contains("exit code 7"));
}

#[test]
fn show_reports_the_persisted_process_exit_code() {
    let response = json!({
        "protocol_version": 2, "type": "RunDetails", "request_id": "tak-runs-show",
        "run": {"summary": {"run_id": "run-1", "state": "failed",
            "created_at_ms": 1, "updated_at_ms": 2, "targets": ["//:exec"],
            "total_jobs": 1, "terminal_jobs": 1, "exit_code": 7}, "jobs": [],
            "logs_expired": false, "outputs_expired": false},
    });
    let output = execute(&["runs", "show", "run-1"], response);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("exit_code=7"));
}

fn execute(arguments: &[&str], response: Value) -> std::process::Output {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("takd.sock");
    let mut bytes = serde_json::to_vec(&response).unwrap();
    bytes.push(b'\n');
    let daemon = FakeRunDaemon::spawn(&socket, Reply::Raw(bytes));
    let environment = BTreeMap::from([("TAKD_SOCKET".into(), socket.display().to_string())]);
    let output = run_tak_output(root.path(), arguments, &environment).unwrap();
    daemon.finish_expecting(if arguments.get(1) == Some(&"attach") {
        2
    } else {
        1
    });
    output
}
