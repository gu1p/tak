use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn exec_rejects_missing_passed_environment_before_submission() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    let socket = PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::SubmissionFlow);
    let env = BTreeMap::from([("TAKD_SOCKET".into(), "../d.sock".into())]);

    let output = run_tak_output(
        &workspace,
        &[
            "exec",
            "--pass-env",
            "TAK_EXEC_CONTRACT_DEFINITELY_MISSING",
            "--",
            "true",
        ],
        &env,
    )
    .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(
        stderr.contains("missing requested environment variables")
            && stderr.contains("TAK_EXEC_CONTRACT_DEFINITELY_MISSING"),
        "{stderr}"
    );
    assert!(daemon.finish_expecting(0).is_empty());
}
