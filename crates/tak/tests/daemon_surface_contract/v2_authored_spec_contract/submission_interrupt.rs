#![cfg(unix)]

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::{tak_bin, write_tasks};

#[test]
fn ctrl_c_after_submission_cancels_a_pending_upload_or_commit() {
    exercise(
        "UploadWorkspace",
        &["SubmitRun", "UploadWorkspace", "CancelRun", "AttachRun"],
    );
    exercise(
        "CommitRun",
        &[
            "SubmitRun",
            "UploadWorkspace",
            "CommitRun",
            "CancelRun",
            "AttachRun",
        ],
    );
}

fn exercise(pending: &'static str, expected: &[&str]) {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let daemon = FakeRunDaemon::spawn(
        &socket,
        Reply::DelayedSubmissionFlow(pending, Duration::from_secs(30)),
    );
    let mut child = Command::new(tak_bin())
        .current_dir(&workspace)
        .args(["run", "//:check"])
        .env("TAKD_SOCKET", "../d.sock")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    wait_for_request(&daemon, expected.len() - 2);
    assert!(
        Command::new("kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let exited = wait_for_exit(&mut child);
    if !exited {
        child.kill().unwrap();
    }
    let output = child.wait_with_output().unwrap();
    let requests = daemon.finish_expecting(expected.len());
    let operations: Vec<_> = requests
        .iter()
        .map(|request| request["operation"]["type"].as_str().unwrap())
        .collect();
    assert_eq!(operations, expected);
    assert_eq!(
        requests[expected.len() - 2]["operation"]["run_id"],
        "run-123"
    );
    assert!(exited, "cancellation waited for the pending {pending}");
    assert!(output.status.code().is_some(), "tak died from SIGINT");
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("run_id=run-123"));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Cancellation persisted"));
}

fn wait_for_exit(child: &mut std::process::Child) -> bool {
    let deadline = Instant::now() + Duration::from_secs(3);
    while child.try_wait().unwrap().is_none() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    child.try_wait().unwrap().is_some()
}

fn wait_for_request(daemon: &FakeRunDaemon, count: usize) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while daemon.request_count() < count {
        assert!(
            Instant::now() < deadline,
            "pending request was not observed"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

const TASKS: &str = r#"SPEC = module_spec(spec_version=2, tasks=[task("check", steps=[cmd("true")])])
SPEC
"#;
