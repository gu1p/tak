use std::collections::BTreeMap;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn a_lost_daemon_connection_reports_unknown_outcome_without_cancelling_work() {
    let root = tempfile::tempdir().expect("temp root");
    let socket = root.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::Close);
    let env = BTreeMap::from([("TAKD_SOCKET".to_string(), socket.display().to_string())]);

    let output =
        run_tak_output(root.path(), &["runs", "cancel", "run-1"], &env).expect("run cancel");
    let requests = daemon.finish_expecting(1);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(requests.len(), 1, "lost request was retried");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(&socket.display().to_string()), "{stderr}");
    assert!(
        stderr.contains("closed before a complete response"),
        "{stderr}"
    );
    assert!(stderr.contains("request outcome is unknown"), "{stderr}");
    assert!(stderr.contains("not a cancellation signal"), "{stderr}");
    assert!(
        stderr.contains("cancellation request may already be persisted"),
        "{stderr}"
    );
    assert!(stderr.contains("tak runs show"), "{stderr}");
    assert!(stderr.contains("tak runs list"), "{stderr}");
    assert!(stderr.contains("tak runs attach"), "{stderr}");
    assert!(stderr.contains("no client execution fallback"), "{stderr}");
    assert!(!stderr.contains("start `takd serve`"), "{stderr}");
    assert!(!stderr.contains("restart"), "{stderr}");
}
