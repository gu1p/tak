use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde_json::json;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn docker_run_submits_a_concrete_container_job_without_client_execution() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(workspace.join("work")).unwrap();
    fs::write(workspace.join("input.txt"), "workspace input").unwrap();
    let socket = PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::SubmissionFlow);
    let env = BTreeMap::from([
        ("TAKD_SOCKET".into(), "../d.sock".into()),
        ("XDG_STATE_HOME".into(), "../state".into()),
        ("DOCKER_TOKEN".into(), "secret".into()),
    ]);

    let output = run_tak_output(
        &workspace,
        &[
            "--local",
            "docker",
            "run",
            "--workdir",
            "work",
            "--env",
            "INLINE=value",
            "--pass-env",
            "DOCKER_TOKEN",
            "alpine:3.20",
            "sh",
            "-c",
            "touch client-executor-ran",
        ],
        &env,
    )
    .unwrap();
    let requests = daemon.finish_expecting(5);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "{stderr}");
    assert!(!workspace.join("client-executor-ran").exists());
    assert_eq!(
        requests
            .iter()
            .map(|request| request["operation"]["type"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "SubmitRun",
            "UploadWorkspace",
            "CommitRun",
            "AttachRun",
            "GetOutputManifest",
        ]
    );
    let submit = &requests[0]["operation"];
    assert_eq!(submit["run"]["targets"], json!(["//:docker-run"]));
    let task = &submit["run"]["tasks"][0];
    assert_eq!(task["runtime"]["kind"], "container");
    assert_eq!(task["runtime"]["source"]["kind"], "image");
    assert_eq!(task["runtime"]["source"]["image"], "alpine:3.20");
    assert_eq!(task["steps"][0]["cwd"], "work");
    assert_eq!(task["steps"][0]["env"], json!({"INLINE": "value"}));
    assert_eq!(task["pass_env_names"], json!(["DOCKER_TOKEN"]));
    assert_eq!(submit["environment_values"].as_array().unwrap().len(), 1);
}
