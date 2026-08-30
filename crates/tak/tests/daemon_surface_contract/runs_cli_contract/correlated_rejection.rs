use std::collections::BTreeMap;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn a_correlated_admission_rejection_reports_that_cancellation_was_not_persisted() {
    let root = tempfile::tempdir().expect("temp root");
    let socket = root.path().join("takd.sock");
    let secret = "DAEMON_REJECTION_SECRET_MUST_NOT_RENDER";
    let response = format!(
        r#"{{"protocol_version":2,"type":"Error","request_id":"tak-runs-cancel","message":"{secret}","code":"protocol_request_invalid","retryable":false}}
"#
    );
    let daemon = FakeRunDaemon::spawn(&socket, Reply::Raw(response.into_bytes()));
    let env = BTreeMap::from([("TAKD_SOCKET".to_string(), socket.display().to_string())]);

    let output = run_tak_output(root.path(), &["runs", "cancel", "run-1"], &env)
        .expect("run cancellation request");
    let requests = daemon.finish_expecting(1);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(requests.len(), 1, "rejected request was retried");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("rejected the protocol v2 request before dispatch"),
        "{stderr}"
    );
    assert!(stderr.contains("was not persisted"), "{stderr}");
    assert!(
        stderr.contains("upgrade tak, takd, and workers together"),
        "{stderr}"
    );
    assert!(stderr.contains("no legacy fallback"), "{stderr}");
    assert!(!stderr.contains(secret), "{stderr}");
    assert!(!stderr.contains("outcome is unknown"), "{stderr}");
    assert!(!stderr.contains("may already be persisted"), "{stderr}");
}
