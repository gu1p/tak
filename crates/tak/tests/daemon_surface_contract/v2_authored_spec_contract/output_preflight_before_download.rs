#![cfg(unix)]

use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use super::second_submission_interrupt::wait_for_requests;
use crate::support::{tak_bin, write_tasks};

#[test]
fn checkout_conflicts_are_reported_before_any_output_chunk_is_downloaded() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    for path in ["first.txt", "second.txt"] {
        std::fs::write(workspace.join(path), "submitted").unwrap();
    }
    let socket = temp.path().join("d.sock");
    let daemon = FakeRunDaemon::spawn(
        &socket,
        Reply::PreflightConflictFlow(Duration::from_millis(750)),
    );
    let stop_watcher = Arc::new(AtomicBool::new(false));
    let stage_seen = Arc::new(AtomicBool::new(false));
    let watcher = watch_for_stage(
        workspace.clone(),
        Arc::clone(&stop_watcher),
        Arc::clone(&stage_seen),
    );
    let child = Command::new(tak_bin())
        .current_dir(&workspace)
        .args(["run", "//:produce"])
        .env("TAKD_SOCKET", "../d.sock")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    wait_for_requests(&daemon, 5);
    for path in ["first.txt", "second.txt"] {
        std::fs::write(workspace.join(path), "local-change").unwrap();
    }

    let output = child.wait_with_output().unwrap();
    stop_watcher.store(true, Ordering::Release);
    watcher.join().unwrap();
    let requests = daemon.finish_expecting(5);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "stderr:\n{stderr}");
    for path in ["first.txt", "second.txt"] {
        assert!(stderr.contains(path), "missing conflict {path}: {stderr}");
        assert_eq!(
            std::fs::read(workspace.join(path)).unwrap(),
            b"local-change"
        );
    }
    assert!(stderr.contains("copied nothing"), "{stderr}");
    assert!(
        requests
            .iter()
            .all(|request| request["operation"]["type"] != "GetOutputChunk")
    );
    assert!(!stage_seen.load(Ordering::Acquire));
    assert!(!has_stage(&workspace));
}

fn watch_for_stage(
    workspace: std::path::PathBuf,
    stop: Arc<AtomicBool>,
    seen: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        while !stop.load(Ordering::Acquire) {
            seen.fetch_or(has_stage(&workspace), Ordering::AcqRel);
            std::thread::sleep(Duration::from_millis(2));
        }
    })
}

fn has_stage(workspace: &std::path::Path) -> bool {
    std::fs::read_dir(workspace).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".tak-output-stage-")
    })
}

const TASKS: &str = r#"SPEC = module_spec(spec_version=2, tasks=[task(
  "produce", outputs=[path("first.txt"), path("second.txt")], steps=[cmd("true")],
)])
SPEC
"#;
