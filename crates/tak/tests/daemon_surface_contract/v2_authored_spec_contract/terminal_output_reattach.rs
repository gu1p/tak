use std::fs;
use std::process::{Command, Stdio};

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use super::second_submission_interrupt::wait_for_requests;
use super::submission_support::{TASKS, environment};
use crate::support::{run_tak_output, tak_bin, write_tasks};

#[test]
fn reattach_materializes_outputs_before_reporting_failed_or_cancelled() {
    for state in ["failed", "cancelled"] {
        exercise(state);
    }
}

fn exercise(state: &'static str) {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    fs::write(workspace.join("input.txt"), "workspace input").unwrap();
    let socket = temp.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::TerminalOutputFlow(state, true));
    let mut env = environment(&socket);
    env.insert(
        "XDG_STATE_HOME".into(),
        temp.path().join("state").display().to_string(),
    );
    let mut child = Command::new(tak_bin())
        .current_dir(&workspace)
        .args(["run", "//:target", "--pass-env", "CLI_TOKEN"])
        .envs(&env)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    wait_for_requests(&daemon, 4);
    child.kill().unwrap();
    child.wait().unwrap();

    let output = run_tak_output(&workspace, &["runs", "attach", "run-123"], &env).unwrap();
    let requests = daemon.finish_expecting(7);

    assert!(
        !output.status.success(),
        "terminal {state} must remain unsuccessful"
    );
    assert_eq!(
        fs::read(workspace.join("generated.txt")).unwrap(),
        b"artifact"
    );
    assert_eq!(requests[5]["operation"]["type"], "GetOutputManifest");
    assert_eq!(requests[6]["operation"]["type"], "GetOutputChunk");
    if state == "failed" {
        assert_eq!(output.status.code(), Some(7));
    } else {
        assert!(String::from_utf8_lossy(&output.stderr).contains("cancelled"));
    }
}
