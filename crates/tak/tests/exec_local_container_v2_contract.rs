use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde_json::json;

use crate::daemon_surface_contract::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn exec_submits_local_container_runtime_override_to_takd() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(workspace.join("docker")).unwrap();
    fs::write(workspace.join("docker/Dockerfile"), "FROM alpine:3.20\n").unwrap();
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
            "exec",
            "--local",
            "--container",
            "--container-dockerfile",
            "docker/Dockerfile",
            "--",
            "true",
        ],
        &env,
    )
    .unwrap();
    let requests = daemon.finish_expecting(5);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        requests[0]["operation"]["run"]["tasks"][0]["runtime"],
        json!({"kind": "container", "source": {
            "kind": "dockerfile", "dockerfile": "docker/Dockerfile",
            "build_context": "docker"
        }})
    );
}
