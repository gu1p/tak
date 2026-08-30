use std::collections::BTreeMap;
use std::path::PathBuf;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn output_retrieval_rejects_symlink_chains_that_escape_after_resolution() {
    let root = tempfile::tempdir_in(".").unwrap();
    let socket = PathBuf::from(root.path().file_name().unwrap()).join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::SymlinkChainOutputFlow);
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
    daemon.finish_expecting(1);

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsafe"));
    assert!(!destination.exists());
}
