#![cfg(unix)]

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::{tak_bin, write_tasks};

#[test]
fn second_ctrl_c_detaches_while_persisted_cancellation_is_pending() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let daemon = FakeRunDaemon::spawn(
        &socket,
        Reply::DelayedCancellationFlow("UploadWorkspace", Duration::from_millis(500), "cancelled"),
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
    let second_sent = Instant::now();
    interrupt(&child);
    let exited = wait_for_exit(&mut child);
    if !exited {
        child.kill().unwrap();
    }
    let output = child.wait_with_output().unwrap();
    let requests = daemon.finish_expecting(3);
    let operations: Vec<_> = requests
        .iter()
        .map(|request| request["operation"]["type"].as_str().unwrap())
        .collect();
    assert_eq!(operations, ["SubmitRun", "UploadWorkspace", "CancelRun"]);
    assert!(exited, "second Ctrl-C did not detach");
    assert!(
        second_sent.elapsed() >= Duration::from_millis(400),
        "detached before takd confirmed persisted cancellation"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("persisted cancellation continues"));
}

pub(crate) fn interrupt(child: &std::process::Child) {
    assert!(
        Command::new("kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
}

pub(crate) fn wait_for_requests(daemon: &FakeRunDaemon, count: usize) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while daemon.request_count() < count {
        assert!(Instant::now() < deadline, "request was not observed");
        std::thread::sleep(Duration::from_millis(5));
    }
}

pub(crate) fn wait_for_exit(child: &mut std::process::Child) -> bool {
    let deadline = Instant::now() + Duration::from_secs(3);
    while child.try_wait().unwrap().is_none() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    child.try_wait().unwrap().is_some()
}

pub(super) const TASKS: &str = r#"SPEC = module_spec(spec_version=2, tasks=[task("check", steps=[cmd("true")])])
SPEC
"#;
