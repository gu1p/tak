use std::collections::BTreeMap;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn runs_list_bounds_and_redacts_untrusted_daemon_responses() {
    let root = tempfile::tempdir().expect("temp root");
    let secret = "DAEMON_RESPONSE_SECRET_MUST_NOT_RENDER";
    let wrong_id = "WRONG_CORRELATION_SECRET_ID_MUST_NOT_RENDER";
    let oversized = vec![b'x'; 64 * 1024 + 1];
    let cases = [
        (
            "malformed",
            Reply::Raw(format!("{{\"protocol_version\":2,\"secret\":\"{secret}\"\n").into_bytes()),
            "protocol mismatch",
        ),
        ("invalid-utf8", Reply::Raw(vec![0xff, b'\n']), "protocol mismatch"),
        (
            "wrong-id",
            Reply::Raw(
                format!(
                    "{{\"protocol_version\":2,\"type\":\"Error\",\"request_id\":\"{wrong_id}\",\"message\":\"{secret}\",\"code\":\"protocol_v2_not_active\",\"retryable\":false}}\n"
                )
                .into_bytes(),
            ),
            "protocol mismatch",
        ),
        ("retryable", Reply::Retryable(secret), "protocol mismatch"),
        ("incomplete", Reply::RawThenStall(oversized), "timeout"),
    ];

    for (name, reply, expected) in cases {
        let socket = root.path().join(format!("{name}.sock"));
        let daemon = FakeRunDaemon::spawn(&socket, reply);
        let env = BTreeMap::from([("TAKD_SOCKET".to_string(), socket.display().to_string())]);
        let output = run_tak_output(root.path(), &["runs", "list"], &env)
            .expect("run list against hostile daemon");
        let requests = daemon.finish_expecting(1);
        assert!(!output.status.success(), "{name} should fail");
        assert!(output.stdout.is_empty(), "{name} wrote stdout");
        assert_eq!(requests.len(), 1, "{name} retried: {requests:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.to_lowercase().contains(expected), "{name}: {stderr}");
        assert!(!stderr.contains(secret), "{name}: {stderr}");
        assert!(!stderr.contains(wrong_id), "{name}: {stderr}");
    }

    let socket = root.path().join("success.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::Success);
    let env = BTreeMap::from([("TAKD_SOCKET".to_string(), socket.display().to_string())]);
    let output = run_tak_output(root.path(), &["runs", "list"], &env).expect("run list success");
    let requests = daemon.finish_expecting(1);
    assert!(output.status.success());
    assert!(output.stdout.is_empty() && output.stderr.is_empty());
    assert_eq!(requests.len(), 1);
}
