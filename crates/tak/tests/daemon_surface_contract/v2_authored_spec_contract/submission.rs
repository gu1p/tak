use std::fs;

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use super::submission_support::{TASKS, assert_requests, environment};
use crate::support::{run_tak_output, write_tasks};

#[test]
fn explicit_v2_submits_uploads_commits_and_attaches_without_client_execution() {
    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).expect("write tasks");
    fs::write(workspace.join("input.txt"), "workspace input").expect("write input");
    let socket = temp.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::SubmissionFlow);

    let output = run_tak_output(
        &workspace,
        &[
            "run",
            "//:target",
            "-j",
            "3",
            "--keep-going",
            "--pass-env",
            "CLI_TOKEN",
        ],
        &environment(&socket),
    )
    .expect("run tak");
    let requests = daemon.finish_expecting(4);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert_requests(&requests);
    for visible in [
        "run-123",
        "//:dep",
        "//:target",
        "queued",
        "running",
        "succeeded",
    ] {
        assert!(stdout.contains(visible), "missing {visible}: {stdout}");
    }
    for secret in ["default-secret", "task-secret", "cli-secret"] {
        assert!(!stdout.contains(secret) && !stderr.contains(secret));
    }
    assert!(!workspace.join("client-executor-ran").exists());
}
