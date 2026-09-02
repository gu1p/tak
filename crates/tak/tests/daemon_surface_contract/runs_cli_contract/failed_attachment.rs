use std::collections::BTreeMap;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn attach_remembers_failure_across_event_pages_and_exits_unsuccessfully() {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::FailedAttachFlow);
    let environment = BTreeMap::from([("TAKD_SOCKET".into(), socket.display().to_string())]);

    let output = run_tak_output(root.path(), &["runs", "attach", "run-1"], &environment).unwrap();
    daemon.finish_expecting(3);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stdout.contains("failed"), "{stdout}");
    assert!(stderr.contains("did not succeed"), "{stderr}");
    assert!(!stderr.contains("protocol mismatch"), "{stderr}");
}
