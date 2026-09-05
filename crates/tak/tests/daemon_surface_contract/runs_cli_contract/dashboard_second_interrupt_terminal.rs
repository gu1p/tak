use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::terminal::{send_terminal_input, spawn_tak_terminal};

#[test]
fn second_raw_ctrl_c_detaches_only_after_cancellation_is_persisted() {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(
        &socket,
        Reply::InteractiveDashboardCancellationFlow(Duration::from_millis(500)),
    );
    let environment = BTreeMap::from([("TAKD_SOCKET".into(), socket.display().to_string())]);
    let mut child = spawn_tak_terminal(
        root.path(),
        &["runs", "attach", "run-dashboard"],
        &environment,
    )
    .unwrap();
    wait_for_requests(&daemon, 2);
    send_terminal_input(&mut child, b"\x03").unwrap();
    wait_for_requests(&daemon, 3);
    let second_sent = Instant::now();
    send_terminal_input(&mut child, b"\x03").unwrap();

    let output = child.wait_with_output().unwrap();
    let requests = daemon.finish_expecting(3);
    let terminal = String::from_utf8_lossy(&output.stdout);
    let operations = requests.iter().map(operation).collect::<Vec<_>>();

    assert!(!output.status.success(), "{terminal:?}");
    assert_eq!(operations, ["GetRun", "AttachRun", "CancelRun"]);
    assert!(second_sent.elapsed() >= Duration::from_millis(400));
    let restored = terminal.find("\u{1b}[?1049l").expect("screen restored");
    let dashboard = &terminal[..restored];
    assert!(
        dashboard.contains("Cancellation") && dashboard.contains("persisted"),
        "dashboard acknowledgement missing: {terminal:?}"
    );
    assert!(
        terminal.contains("persisted cancellation continues"),
        "{terminal:?}"
    );
}

fn operation(request: &serde_json::Value) -> &str {
    request["operation"]["type"].as_str().unwrap()
}

fn wait_for_requests(daemon: &FakeRunDaemon, count: usize) {
    let deadline = Instant::now() + Duration::from_secs(15);
    while daemon.request_count() < count && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(daemon.request_count(), count, "terminal request deadline");
}
