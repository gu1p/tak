use std::collections::BTreeMap;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn show_and_cancel_reject_success_frames_for_a_different_run() {
    let root = tempfile::tempdir().unwrap();
    let cases = [
        (
            "show",
            vec!["runs", "show", "run-1"],
            r#"{"protocol_version":2,"type":"RunDetails","request_id":"tak-runs-show","run":{"summary":{"run_id":"run-other","state":"running","created_at_ms":1,"updated_at_ms":1,"targets":[],"total_jobs":0,"terminal_jobs":0},"jobs":[]}}
"#,
        ),
        (
            "cancel",
            vec!["runs", "cancel", "run-1"],
            r#"{"protocol_version":2,"type":"CancellationAccepted","request_id":"tak-runs-cancel","run_id":"run-other","state":"cancelling"}
"#,
        ),
    ];
    for (name, args, response) in cases {
        let socket = root.path().join(format!("{name}.sock"));
        let daemon = FakeRunDaemon::spawn(&socket, Reply::Raw(response.as_bytes().to_vec()));
        let environment = BTreeMap::from([("TAKD_SOCKET".into(), socket.display().to_string())]);
        let output = run_tak_output(root.path(), &args, &environment).unwrap();
        daemon.finish_expecting(1);
        assert!(!output.status.success(), "{name} accepted another run");
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .to_lowercase()
                .contains("protocol mismatch")
        );
    }
}
