use std::collections::BTreeMap;
use std::time::Duration;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn a_daemon_timeout_detaches_without_cancelling_daemon_owned_work() {
    let root = tempfile::tempdir().expect("temp root");
    let socket = root.path().join("takd.sock");
    let secret = "SLOW_DAEMON_RESPONSE_MUST_NOT_RENDER";
    let daemon = FakeRunDaemon::spawn(
        &socket,
        Reply::SlowDripInactive(secret, Duration::from_millis(250), 12),
    );
    let env = BTreeMap::from([("TAKD_SOCKET".to_string(), socket.display().to_string())]);

    let output =
        run_tak_output(root.path(), &["runs", "attach", "run-1"], &env).expect("run attach");
    let requests = daemon.finish_expecting(1);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(requests.len(), 1, "timed-out request was retried");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("client timeout"), "{stderr}");
    assert!(stderr.contains("request outcome is unknown"), "{stderr}");
    assert!(stderr.contains("not a cancellation signal"), "{stderr}");
    assert!(
        !stderr.contains("cancellation request may already be persisted"),
        "{stderr}"
    );
    assert!(stderr.contains("tak runs show"), "{stderr}");
    assert!(stderr.contains("tak runs list"), "{stderr}");
    assert!(stderr.contains("tak runs attach"), "{stderr}");
    assert!(stderr.contains("no client execution fallback"), "{stderr}");
    assert!(!stderr.contains(secret), "{stderr}");
    assert!(!stderr.contains("start `takd serve`"), "{stderr}");
    assert!(!stderr.contains("restart"), "{stderr}");
}
