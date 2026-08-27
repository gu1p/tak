#![cfg(unix)]

use std::fs;

use crate::support::{self, bounded_log};

#[test]
fn logs_tail_reads_from_end_of_sparse_service_log() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_root = temp.path().join("state");
    fs::create_dir_all(&state_root).expect("create state root");
    bounded_log::write_sparse_log(
        &state_root.join("service.log"),
        "discard-me\nlast-one\nlast-two\n",
    );

    let output = bounded_log::command_with_data_limit(&support::takd_bin())
        .args([
            "logs",
            "--state-root",
            &state_root.display().to_string(),
            "--lines",
            "2",
        ])
        .output()
        .expect("run memory-bounded takd logs");

    assert!(
        output.status.success(),
        "tailing a sparse log should not allocate the whole file:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "last-one\nlast-two\n"
    );
}
