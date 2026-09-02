use std::fs;

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use super::submission_support::{TASKS, environment};
use crate::support::{run_tak_output, write_tasks};

#[test]
fn foreground_materializes_outputs_before_reporting_failed_or_cancelled() {
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
    let daemon = FakeRunDaemon::spawn(&socket, Reply::TerminalOutputFlow(state, false));
    let mut env = environment(&socket);
    env.insert(
        "XDG_STATE_HOME".into(),
        temp.path().join("state").display().to_string(),
    );

    let output = run_tak_output(
        &workspace,
        &["run", "//:target", "--pass-env", "CLI_TOKEN"],
        &env,
    )
    .unwrap();
    let requests = daemon.finish_expecting(6);

    assert!(
        !output.status.success(),
        "terminal {state} must remain unsuccessful"
    );
    assert_eq!(
        fs::read(workspace.join("generated.txt")).unwrap(),
        b"artifact"
    );
    assert_eq!(requests[4]["operation"]["type"], "GetOutputManifest");
    assert_eq!(requests[5]["operation"]["type"], "GetOutputChunk");
    if state == "failed" {
        assert_eq!(output.status.code(), Some(7));
    } else {
        assert!(String::from_utf8_lossy(&output.stderr).contains("cancelled"));
    }
}
