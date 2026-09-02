use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde_json::json;

use super::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn exec_submits_one_concrete_local_protocol_v2_run() {
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
        ("EXEC_ALPHA".into(), "alpha-secret".into()),
        ("EXEC_BETA".into(), "beta-secret".into()),
    ]);

    let output = run_tak_output(
        &workspace,
        &[
            "exec",
            "--cwd",
            "work",
            "--env",
            "INLINE=value",
            "--pass-env",
            "EXEC_ALPHA",
            "--pass-env",
            "EXEC_BETA",
            "--",
            "sh",
            "-c",
            "touch client-executor-ran",
        ],
        &env,
    )
    .unwrap();
    let requests = daemon.finish_expecting(5);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "{stdout}\n{stderr}");
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
    assert!(
        requests
            .iter()
            .all(|request| request["protocol_version"] == 2)
    );
    let submit = &requests[0]["operation"];
    assert_eq!(submit["run"]["targets"], json!(["//:exec"]));
    assert_eq!(submit["run"]["tasks"].as_array().unwrap().len(), 1);
    let task = &submit["run"]["tasks"][0];
    assert_eq!(task["task_id"], "//:exec");
    assert_eq!(task["timeout_s"], serde_json::Value::Null);
    assert_eq!(task["runtime"], serde_json::Value::Null);
    assert_eq!(task["idempotent"], false);
    assert_eq!(task["steps"][0]["cwd"], "work");
    assert_eq!(task["steps"][0]["env"], json!({"INLINE": "value"}));
    assert_eq!(
        task["steps"][0]["argv"],
        json!(["sh", "-c", "touch client-executor-ran"])
    );
    assert_eq!(task["pass_env_names"], json!(["EXEC_ALPHA", "EXEC_BETA"]));
    assert_eq!(
        submit["run"]["jobs"][0]["placement_candidates"][0]["kind"],
        "local"
    );
    assert_eq!(submit["environment_values"].as_array().unwrap().len(), 2);
    assert!(stdout.contains("run_id=run-123"), "{stdout}");
    assert!(!stdout.contains("alpha-secret") && !stderr.contains("beta-secret"));
}
