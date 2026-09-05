use std::collections::BTreeMap;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::terminal::run_tak_terminal;

#[test]
fn terminal_attach_keeps_the_expired_log_notice_in_final_scrollback() {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::ExpiredDashboardAttachFlow);
    let environment = BTreeMap::from([("TAKD_SOCKET".into(), socket.display().to_string())]);
    let output = run_tak_terminal(
        root.path(),
        &["runs", "attach", "run-dashboard"],
        &environment,
    )
    .unwrap();
    let request_count = daemon.request_count();
    daemon.finish_expecting(request_count);
    let terminal = String::from_utf8_lossy(&output.stdout);
    let restored = terminal
        .rsplit_once("\u{1b}[?1049l")
        .map(|(_, restored)| restored)
        .unwrap_or_default();

    assert!(output.status.success(), "{terminal:?}");
    assert!(
        restored.contains("Run logs have expired."),
        "expired-log notice must remain visible after restoring the terminal: {terminal:?}"
    );
}
