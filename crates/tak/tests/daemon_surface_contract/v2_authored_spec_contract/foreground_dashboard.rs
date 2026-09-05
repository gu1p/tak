use std::fs;

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use super::submission_support::{TASKS, assert_requests, environment};
use crate::support::{terminal::run_tak_terminal, write_tasks};

#[test]
fn terminal_run_shows_the_same_daemon_owned_dashboard_and_restores_the_terminal() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    fs::write(workspace.join("input.txt"), "workspace input").unwrap();
    let socket = temp.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::SubmissionFlow);
    let output = run_tak_terminal(
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
    .unwrap();
    let requests = daemon.finish_expecting(5);
    let terminal = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "{terminal:?}");
    assert_requests(&requests);
    for text in [
        "TAK",
        "RUN",
        "NODES",
        "TASKS",
        "run-123",
        "staging",
        "//:dep",
        "//:target",
        "local",
    ] {
        assert!(terminal.contains(text), "missing {text:?} in {terminal:?}");
    }
    assert!(
        terminal.contains("\u{1b}[?1049h"),
        "alternate screen not entered"
    );
    assert!(
        terminal.contains("\u{1b}[?1049l"),
        "terminal was not restored"
    );
    assert!(!workspace.join("client-executor-ran").exists());
}
