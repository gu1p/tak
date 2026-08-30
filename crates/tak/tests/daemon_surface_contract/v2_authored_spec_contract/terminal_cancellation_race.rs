#![cfg(unix)]

use std::process::{Command, Stdio};
use std::time::Duration;

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use super::second_submission_interrupt::{TASKS, interrupt, wait_for_exit, wait_for_requests};
use crate::support::{tak_bin, write_tasks};

#[test]
fn second_ctrl_c_does_not_claim_persisted_cancellation_when_run_is_terminal() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let daemon = FakeRunDaemon::spawn(
        &socket,
        Reply::DelayedCancellationFlow("UploadWorkspace", Duration::from_millis(500), "succeeded"),
    );
    let mut child = Command::new(tak_bin())
        .current_dir(&workspace)
        .args(["run", "//:check"])
        .env("TAKD_SOCKET", "../d.sock")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    wait_for_requests(&daemon, 2);
    interrupt(&child);
    wait_for_requests(&daemon, 3);
    interrupt(&child);
    assert!(
        wait_for_exit(&mut child),
        "terminal run did not finish attachment"
    );
    let output = child.wait_with_output().unwrap();
    let requests = daemon.finish_expecting(4);
    let operations: Vec<_> = requests
        .iter()
        .map(|request| request["operation"]["type"].as_str().unwrap())
        .collect();
    assert_eq!(
        operations,
        ["SubmitRun", "UploadWorkspace", "CancelRun", "AttachRun"]
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    assert!(stderr.contains("already terminal"), "{stderr}");
    assert!(
        !stderr.contains("persisted cancellation continues"),
        "{stderr}"
    );
}
