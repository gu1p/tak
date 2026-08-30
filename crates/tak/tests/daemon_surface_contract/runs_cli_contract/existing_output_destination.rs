use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn output_retrieval_refuses_an_existing_destination_before_downloading() {
    let root = tempfile::tempdir_in(".").expect("temp root");
    let socket = PathBuf::from(root.path().file_name().unwrap()).join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::ManagementFlow);
    let environment = BTreeMap::from([("TAKD_SOCKET".into(), "takd.sock".into())]);
    let destination = root.path().join("outputs");
    fs::create_dir(&destination).unwrap();
    fs::write(destination.join("keep.txt"), "keep").unwrap();
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
    assert!(String::from_utf8_lossy(&output.stderr).contains("already exists"));
    assert_eq!(
        fs::read_to_string(destination.join("keep.txt")).unwrap(),
        "keep"
    );
    assert_eq!(fs::read_dir(destination).unwrap().count(), 1);
}
