use std::collections::BTreeMap;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn attach_rejects_a_daemon_page_that_skips_a_persisted_event() {
    let root = tempfile::tempdir().unwrap();
    let socket = root.path().join("takd.sock");
    let response = br#"{"protocol_version":2,"type":"RunEvents","request_id":"tak-runs-attach","run_id":"run-1","events":[{"seq":2,"kind":"succeeded","job_id":null,"task_ids":[],"node_id":null,"message":"done"}],"next_event":2,"state":"succeeded","terminal":true}
"#;
    let daemon = FakeRunDaemon::spawn(&socket, Reply::Raw(response.to_vec()));
    let environment = BTreeMap::from([("TAKD_SOCKET".into(), socket.display().to_string())]);

    let output = run_tak_output(root.path(), &["runs", "attach", "run-1"], &environment).unwrap();
    daemon.finish_expecting(1);

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .to_lowercase()
            .contains("protocol mismatch")
    );
}
