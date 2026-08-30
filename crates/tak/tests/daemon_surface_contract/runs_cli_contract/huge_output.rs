use std::collections::BTreeMap;
use std::path::PathBuf;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn daemon_declared_output_size_cannot_panic_or_preallocate_unbounded_memory() {
    let root = tempfile::tempdir_in(".").unwrap();
    let socket = PathBuf::from(root.path().file_name().unwrap()).join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::HugeOutputFlow);
    let environment = BTreeMap::from([("TAKD_SOCKET".into(), "takd.sock".into())]);
    let destination = root.path().join("outputs");
    let output = run_tak_output(
        root.path(),
        &[
            "runs",
            "outputs",
            "run-1",
            "--to",
            destination.to_str().unwrap(),
        ],
        &environment,
    )
    .unwrap();
    daemon.finish_expecting(2);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(stderr.contains("invalid"), "{stderr}");
    assert!(!stderr.contains("panicked") && !stderr.contains("capacity overflow"));
    assert!(!destination.exists());
}
