use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::super::v2_authored_spec_contract::second_submission_interrupt::{
    interrupt, wait_for_exit, wait_for_requests,
};
use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::tak_bin;

#[test]
fn first_attach_interrupt_persists_cancel_and_follows_it_to_cancelled() {
    let temp = tempfile::tempdir().unwrap();
    let socket = temp.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::AttachCancellationFlow(Duration::ZERO));
    let mut child = attach(&socket, temp.path());
    wait_for_requests(&daemon, 1);
    interrupt(&child);

    assert!(
        wait_for_exit(&mut child),
        "attach did not follow cancellation to terminal state"
    );
    let output = child.wait_with_output().unwrap();
    let requests = daemon.finish_expecting(5);
    assert_eq!(
        operations(&requests),
        [
            "AttachRun",
            "CancelRun",
            "AttachRun",
            "AttachRun",
            "GetOutputManifest"
        ]
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Cancellation persisted for run-123"),
        "{stderr}"
    );
    assert!(stderr.contains("run run-123 was cancelled"), "{stderr}");
}

#[test]
fn second_attach_interrupt_detaches_only_after_cancel_is_persisted() {
    let temp = tempfile::tempdir().unwrap();
    let socket = temp.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(
        &socket,
        Reply::AttachCancellationFlow(Duration::from_millis(500)),
    );
    let mut child = attach(&socket, temp.path());
    wait_for_requests(&daemon, 1);
    interrupt(&child);
    wait_for_requests(&daemon, 2);
    let second_sent = Instant::now();
    interrupt(&child);

    assert!(wait_for_exit(&mut child), "second interrupt did not detach");
    let output = child.wait_with_output().unwrap();
    let requests = daemon.finish_expecting(2);
    assert_eq!(operations(&requests), ["AttachRun", "CancelRun"]);
    assert!(second_sent.elapsed() >= Duration::from_millis(400));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Cancellation persisted for run-123"),
        "{stderr}"
    );
    assert!(
        stderr.contains("persisted cancellation continues"),
        "{stderr}"
    );
}

fn attach(socket: &std::path::Path, cwd: &std::path::Path) -> std::process::Child {
    Command::new(tak_bin())
        .current_dir(cwd)
        .args(["runs", "attach", "run-123"])
        .env("TAKD_SOCKET", socket)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap()
}

fn operations(requests: &[serde_json::Value]) -> Vec<&str> {
    requests
        .iter()
        .map(|request| request["operation"]["type"].as_str().unwrap())
        .collect()
}
