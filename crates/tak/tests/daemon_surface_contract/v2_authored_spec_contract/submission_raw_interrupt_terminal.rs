#![cfg(unix)]

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::{terminal::send_terminal_input, terminal::spawn_tak_terminal, write_tasks};

#[test]
fn raw_ctrl_c_cancels_while_workspace_upload_or_commit_is_pending() {
    exercise("UploadWorkspace", 2);
    exercise("CommitRun", 3);
}

fn exercise(pending: &'static str, pending_request_count: usize) {
    let root = tempfile::tempdir().unwrap();
    let workspace = root.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    let socket = root.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(
        &socket,
        Reply::DelayedSubmissionFlow(pending, Duration::from_secs(30)),
    );
    let environment = BTreeMap::from([
        ("TAKD_SOCKET".into(), socket.display().to_string()),
        (
            "XDG_STATE_HOME".into(),
            root.path().join("state").display().to_string(),
        ),
    ]);
    let mut child = spawn_tak_terminal(&workspace, &["run", "//:check"], &environment).unwrap();

    wait_for_requests(&daemon, pending_request_count);
    send_terminal_input(&mut child, b"\x03").unwrap();
    let output = child.wait_with_output().unwrap();
    let requests = daemon.finish_expecting(pending_request_count + 3);
    let operations = requests.iter().map(operation).collect::<Vec<_>>();
    let terminal = String::from_utf8_lossy(&output.stdout);

    assert!(!output.status.success(), "{pending}: {terminal:?}");
    assert_eq!(
        operations[pending_request_count..],
        ["CancelRun", "AttachRun", "GetOutputManifest"]
    );
    let restored = terminal.find("\u{1b}[?1049l").expect("screen restored");
    let acknowledgement = terminal.rfind("Cancellation persisted").unwrap();
    assert!(acknowledgement > restored, "{pending}: {terminal:?}");
}

fn operation(request: &serde_json::Value) -> &str {
    request["operation"]["type"].as_str().unwrap()
}

fn wait_for_requests(daemon: &FakeRunDaemon, count: usize) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while daemon.request_count() < count && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(daemon.request_count(), count, "pending request deadline");
}

const TASKS: &str = r#"SPEC = module_spec(
  spec_version=2,
  tasks=[task("check", steps=[cmd("true")])],
)
SPEC
"#;
