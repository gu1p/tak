use std::fs;

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use super::submission_support::{TASKS, environment};
use crate::support::{run_tak_output, write_tasks};

#[test]
fn foreground_v2_run_preserves_failure_across_attachment_pages() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    fs::write(workspace.join("input.txt"), "workspace input").unwrap();
    let socket = temp.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::FailedSubmissionFlow);

    let output = run_tak_output(
        &workspace,
        &["run", "//:target", "--pass-env", "CLI_TOKEN"],
        &environment(&socket),
    )
    .unwrap();
    daemon.finish_expecting(5);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stdout.contains("run-123") && stdout.contains("failed"));
    assert!(stderr.contains("did not succeed"), "{stderr}");
}
