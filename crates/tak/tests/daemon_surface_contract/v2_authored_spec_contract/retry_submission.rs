use std::fs;

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use super::submission_support::{TASKS, environment};
use crate::support::{run_tak_output, write_tasks};

#[test]
fn ambiguous_submit_retries_with_the_same_idempotency_key() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    fs::write(workspace.join("input.txt"), "workspace input").unwrap();
    let socket = temp.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::RetrySubmissionFlow);

    let output = run_tak_output(
        &workspace,
        &["run", "//:target", "--pass-env", "CLI_TOKEN"],
        &environment(&socket),
    )
    .unwrap();
    let requests = daemon.finish_expecting(6);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(requests[0]["operation"]["type"], "SubmitRun");
    assert_eq!(requests[1]["operation"]["type"], "SubmitRun");
    assert_eq!(
        requests[0]["operation"]["idempotency_key"],
        requests[1]["operation"]["idempotency_key"]
    );
}
