use std::collections::BTreeMap;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn redirected_attach_stays_append_only_accessible_and_ansi_free() {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::DashboardAttachFlow);
    let env = BTreeMap::from([("TAKD_SOCKET".into(), socket.display().to_string())]);
    let output = run_tak_output(root.path(), &["runs", "attach", "run-dashboard"], &env).unwrap();
    let requests = daemon.finish_expecting(3);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "{stdout}\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        stdout.contains("queued tasks=//:lint node=- waiting for capacity"),
        "{stdout}"
    );
    assert!(
        stdout.contains("running tasks=//:build node=worker-a started"),
        "{stdout}"
    );
    assert!(
        stdout.contains("build log\n") && stdout.contains("succeeded tasks=//:test"),
        "{stdout}"
    );
    assert!(!stdout.contains('\u{1b}') && !output.stderr.contains(&0x1b));
    assert_eq!(requests[0]["operation"]["type"], "AttachRun");
}
