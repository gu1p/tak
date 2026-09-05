use std::collections::BTreeMap;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::terminal::run_tak_terminal;

#[test]
fn terminal_attach_shows_the_daemon_owned_run_dashboard_and_restores_the_terminal() {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::DashboardAttachFlow);
    let environment = BTreeMap::from([("TAKD_SOCKET".into(), socket.display().to_string())]);
    let output = run_tak_terminal(
        root.path(),
        &["runs", "attach", "run-dashboard"],
        &environment,
    )
    .unwrap();
    let request_count = daemon.request_count();
    let requests = daemon.finish_expecting(request_count);
    let terminal = String::from_utf8_lossy(&output.stdout);
    let operations = requests
        .iter()
        .map(|value| value["operation"]["type"].as_str().unwrap())
        .collect::<Vec<_>>();

    assert!(output.status.success(), "{terminal:?}");
    assert_eq!(&operations[..2], ["GetRun", "AttachRun"]);
    for text in [
        "TAK",
        "RUN",
        "NODES",
        "SCHEDULER",
        "QUEUE",
        "TASKS",
        "LIVE",
        "LOGS",
        "worker-a",
        "worker-b",
        "worker-c",
        "//:build",
        "//:test",
        "//:lint",
        "2/3",
        "queue:",
        "builds",
        "candidates:",
        "log",
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
}
