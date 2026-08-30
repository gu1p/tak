use std::collections::BTreeMap;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn outputs_requires_a_destination_and_never_creates_it_before_a_valid_response() {
    let root = tempfile::tempdir().expect("temp root");
    let destination = root.path().join("absent-output-destination");
    let destination_arg = destination.display().to_string();
    let socket = root.path().join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::Inactive("untrusted"));
    let env = BTreeMap::from([("TAKD_SOCKET".to_string(), socket.display().to_string())]);

    let usage = run_tak_output(root.path(), &["runs", "outputs", "run-1"], &env)
        .expect("run outputs without destination");
    assert!(!usage.status.success());
    assert!(usage.stdout.is_empty());
    assert!(String::from_utf8_lossy(&usage.stderr).contains("--to"));

    let inactive = run_tak_output(
        root.path(),
        &["runs", "outputs", "run-1", "--to", &destination_arg],
        &env,
    )
    .expect("run outputs against inactive daemon");
    let requests = daemon.finish_expecting(1);
    assert!(!inactive.status.success());
    assert!(inactive.stdout.is_empty());
    assert_eq!(requests.len(), 1, "usage failure contacted daemon");
    assert!(
        !destination.exists(),
        "inactive command created destination"
    );
}
