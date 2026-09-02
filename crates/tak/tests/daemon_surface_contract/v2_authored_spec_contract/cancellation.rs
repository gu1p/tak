#![cfg(unix)]

use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use tak_proto::local_daemon::v2::RunLifecycleState;
use takd::RunStore;

use crate::support::local_daemon::LocalDaemonGuard;
use crate::support::{tak_bin, write_tasks};

#[path = "cancellation_support.rs"]
mod support;

#[test]
fn first_interrupt_persists_cancellation_and_waits_for_daemon_confirmation() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, support::TASKS).unwrap();
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let daemon = LocalDaemonGuard::spawn(&socket, &support::empty_spec(&workspace));
    let store = RunStore::with_db_path(daemon.db_path().to_path_buf()).unwrap();
    let probe = temp.path().canonicalize().unwrap().join("probe");
    let mut child = Command::new(tak_bin())
        .current_dir(&workspace)
        .args(["run", "//:slow", "--pass-env", "PROBE"])
        .env("TAKD_SOCKET", "../d.sock")
        .env("PROBE", &probe)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let run_id = support::wait_for_running(&store);
    support::wait_for_probe(&probe);
    assert!(
        Command::new("kill")
            .args(["-INT", &child.id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.try_wait().unwrap().is_none() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let output = child.wait_with_output().unwrap();
    let cancelled = support::wait_for_terminal(&store, &run_id) == RunLifecycleState::Cancelled;
    if !cancelled {
        store.cancel(&run_id).unwrap();
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(cancelled, "Ctrl-C did not persist cancellation: {stdout}");
    assert!(
        stdout.contains("cancelling") && stdout.contains("cancelled"),
        "{stdout}"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(&format!("Cancellation persisted for {run_id}")),
        "{stderr}"
    );
    assert_eq!(std::fs::read_to_string(probe).unwrap(), "started");
}
