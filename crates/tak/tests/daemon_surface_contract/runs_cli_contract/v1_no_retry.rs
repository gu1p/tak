use std::collections::BTreeMap;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn cancelling_rejects_a_v1_response_without_retry_or_legacy_fallback() {
    let root = tempfile::tempdir().expect("temp root");
    let socket = root.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(
        &socket,
        Reply::Legacy("LEGACY_RESPONSE_MUST_NOT_BE_TRUSTED"),
    );
    let env = BTreeMap::from([("TAKD_SOCKET".to_string(), socket.display().to_string())]);
    let output =
        run_tak_output(root.path(), &["runs", "cancel", "run-1"], &env).expect("run cancellation");
    let requests = daemon.finish_expecting(1);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(requests.len(), 1, "v1 must not trigger a second request");
    assert_eq!(requests[0]["protocol_version"], 2);
    assert_eq!(requests[0]["operation"]["type"], "CancelRun");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("protocol mismatch"), "{stderr}");
    assert!(
        stderr.contains("upgrade tak, takd, and workers together"),
        "{stderr}"
    );
    assert!(
        stderr.contains("No legacy fallback was attempted"),
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
    assert!(!stderr.contains("start `takd serve`"), "{stderr}");
    assert!(!stderr.contains("restart"), "{stderr}");
    assert!(!stderr.contains("LEGACY_RESPONSE_MUST_NOT_BE_TRUSTED"));
}
