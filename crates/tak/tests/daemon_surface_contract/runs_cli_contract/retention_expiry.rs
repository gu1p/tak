use std::collections::BTreeMap;

use serde_json::{Value, json};

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn show_reports_expired_log_and_output_payloads() {
    let response = json!({
        "protocol_version": 2, "type": "RunDetails", "request_id": "tak-runs-show",
        "run": {"summary": summary("succeeded"), "jobs": [],
            "logs_expired": true, "outputs_expired": true},
    });
    let (_root, output) = execute(&["runs", "show", "run-1"], response);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("logs=expired"), "{stdout}");
    assert!(stdout.contains("outputs=expired"), "{stdout}");
}

#[test]
fn attach_reports_expired_logs_without_rendering_a_protocol_error() {
    let response = json!({
        "protocol_version": 2, "type": "RunEvents", "request_id": "tak-runs-attach",
        "run_id": "run-1", "events": [], "next_event": 7, "state": "failed",
        "terminal": true, "logs_expired": true,
    });
    let (_root, output) = execute(&["runs", "attach", "run-1"], response);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("Run logs have expired."), "{stderr}");
    assert!(stderr.contains("did not succeed"), "{stderr}");
    assert!(!stderr.contains("protocol mismatch"), "{stderr}");
}

#[test]
fn outputs_refuses_expired_artifacts_before_creating_the_destination() {
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("outputs");
    let destination_arg = destination.display().to_string();
    let response = json!({
        "protocol_version": 2, "type": "OutputManifest", "request_id": "tak-runs-outputs",
        "run_id": "run-1", "expired": true, "artifacts": [],
    });
    let output = execute_in(
        &root,
        &["runs", "outputs", "run-1", "--to", &destination_arg],
        response,
    );
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("artifacts have expired"));
    assert!(!destination.exists());
}

fn execute(arguments: &[&str], response: Value) -> (tempfile::TempDir, std::process::Output) {
    let root = tempfile::tempdir().unwrap();
    let output = execute_in(&root, arguments, response);
    (root, output)
}

fn execute_in(
    root: &tempfile::TempDir,
    arguments: &[&str],
    response: Value,
) -> std::process::Output {
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

fn summary(state: &str) -> Value {
    json!({"run_id": "run-1", "state": state, "created_at_ms": 1, "updated_at_ms": 2,
        "targets": ["//:check"], "total_jobs": 1, "terminal_jobs": 1})
}
