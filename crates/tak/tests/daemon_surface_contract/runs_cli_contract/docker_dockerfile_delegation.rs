use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde_json::json;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn dockerfile_run_submits_workspace_paths_and_command_to_takd() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir(&workspace).unwrap();
    fs::write(workspace.join("Dockerfile"), "FROM alpine:3.20\n").unwrap();
    let socket = PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::SubmissionFlow);
    let env = BTreeMap::from([
        ("TAKD_SOCKET".into(), "../d.sock".into()),
        ("XDG_STATE_HOME".into(), "../state".into()),
    ]);

    let output = run_tak_output(
        &workspace,
        &[
            "--local",
            "docker",
            "run",
            "-f",
            "Dockerfile",
            "--build-context",
            ".",
            "sh",
            "-c",
            "printf delegated",
        ],
        &env,
    )
    .unwrap();
    let requests = daemon.finish_expecting(5);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "{stderr}");
    let task = &requests[0]["operation"]["run"]["tasks"][0];
    assert_eq!(
        task["runtime"]["source"],
        json!({
            "kind": "dockerfile",
            "dockerfile": "Dockerfile",
            "build_context": "."
        })
    );
    assert_eq!(
        task["steps"][0]["argv"],
        json!(["sh", "-c", "printf delegated"])
    );
}
