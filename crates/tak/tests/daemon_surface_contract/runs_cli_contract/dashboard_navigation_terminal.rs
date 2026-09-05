use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::terminal::{send_terminal_input, spawn_tak_terminal};

#[test]
fn terminal_navigation_reaches_late_tasks_without_waiting_for_daemon_events() {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(
        &socket,
        Reply::InteractiveDashboardFlow(Duration::from_millis(600)),
    );
    let environment = BTreeMap::from([("TAKD_SOCKET".into(), socket.display().to_string())]);
    let mut child = spawn_tak_terminal(
        root.path(),
        &["runs", "attach", "run-dashboard"],
        &environment,
    )
    .unwrap();
    wait_for_requests(&daemon, 2);
    send_terminal_input(&mut child, b"\x1b[F").unwrap();
    let output = child.wait_with_output().unwrap();
    let requests = daemon.finish_expecting(3);
    let terminal = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success(), "{terminal:?}");
    assert!(
        terminal.contains("FINAL-TASK-REACHED"),
        "End did not reveal the last task: {terminal:?}"
    );
    assert_eq!(operation(&requests[0]), "GetRun");
    assert_eq!(operation(&requests[1]), "AttachRun");
}

fn operation(request: &serde_json::Value) -> &str {
    request["operation"]["type"].as_str().unwrap()
}

fn wait_for_requests(daemon: &FakeRunDaemon, count: usize) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while daemon.request_count() < count && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        daemon.request_count(),
        count,
        "terminal did not reach attach"
    );
}
