use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::{tak_bin, write_tasks};

#[test]
fn foreground_run_rechecks_the_checkout_after_downloading_outputs() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    std::fs::write(workspace.join("generated.txt"), "submitted").unwrap();
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let daemon = FakeRunDaemon::spawn(
        &socket,
        Reply::DelayedOutputSubmissionFlow(Duration::from_millis(750)),
    );
    let child = Command::new(tak_bin())
        .current_dir(&workspace)
        .args(["run", "//:check"])
        .env("TAKD_SOCKET", "../d.sock")
        .env("XDG_STATE_HOME", "../state")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    wait_for_chunk_request(&daemon);
    std::fs::write(workspace.join("generated.txt"), "local-change").unwrap();
    let output = child.wait_with_output().unwrap();
    let requests = daemon.finish_expecting(6);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success(), "stderr:\n{stderr}");
    assert!(stderr.contains("generated.txt"), "{stderr}");
    assert!(stderr.contains("copied nothing"), "{stderr}");
    assert_eq!(
        std::fs::read(workspace.join("generated.txt")).unwrap(),
        b"local-change"
    );
    assert_eq!(requests[5]["operation"]["type"], "GetOutputChunk");
}

fn wait_for_chunk_request(daemon: &FakeRunDaemon) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while daemon.request_count() < 6 {
        assert!(
            Instant::now() < deadline,
            "output chunk was not requested; received {} requests",
            daemon.request_count()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

const TASKS: &str = r#"SPEC = module_spec(spec_version=2, tasks=[task(
  "check", outputs=[path("generated.txt")], steps=[cmd("true")],
)])
SPEC
"#;
