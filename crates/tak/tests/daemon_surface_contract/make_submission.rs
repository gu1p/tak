use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde_json::json;

use super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::run_tak_output;

#[test]
fn make_submits_its_resolved_parallel_graph_to_takd() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    fs::create_dir_all(&workspace).unwrap();
    fs::write(
        workspace.join("Makefile"),
        ".PHONY: all left right\n# tak: parallel=left,right\n\
         all: left right\n\t@touch client-parent\n\
         left:\n\t@touch client-left\nright:\n\t@touch client-right\n",
    )
    .unwrap();
    let socket = PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::SubmissionFlow);
    let environment = BTreeMap::from([
        ("TAKD_SOCKET".into(), "../d.sock".into()),
        ("MAKE_TOKEN".into(), "secret".into()),
    ]);

    let output = run_tak_output(
        &workspace,
        &["make", "all", "--pass-env", "MAKE_TOKEN"],
        &environment,
    )
    .unwrap();
    let requests = daemon.finish_expecting(5);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "{stderr}");
    assert!(!workspace.join("client-parent").exists());
    let operations = requests
        .iter()
        .map(|request| request["operation"]["type"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        operations,
        [
            "SubmitRun",
            "UploadWorkspace",
            "CommitRun",
            "AttachRun",
            "GetOutputManifest",
        ]
    );
    let submit = &requests[0]["operation"];
    let run = &submit["run"];
    assert_eq!(run["targets"], json!(["//:make-2"]));
    assert_eq!(
        run["options"],
        json!({"max_parallel_jobs": 3, "keep_going": true})
    );
    assert_eq!(run["tasks"].as_array().unwrap().len(), 3);
    assert_eq!(run["jobs"].as_array().unwrap().len(), 3);
    assert_eq!(run["job_edges"].as_array().unwrap().len(), 2);
    assert!(run["jobs"].as_array().unwrap().iter().all(|job| {
        job["session"]["reuse"] == json!({"kind": "shared_workspace", "max_parallel_tasks": 3})
            && job["affinity"] == json!({"kind": "require_same_node", "group": "tak-make"})
    }));
    assert!(
        run["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .all(|task| { task["pass_env_names"] == json!(["MAKE_TOKEN"]) })
    );
    assert_eq!(run["tasks"][0]["outputs"], json!([]));
    assert_eq!(run["tasks"][1]["outputs"], json!([]));
    assert_eq!(
        run["tasks"][2]["outputs"],
        json!([{"kind": "glob", "value": "**"}])
    );
    assert_eq!(submit["environment_values"][0]["name"], "MAKE_TOKEN");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("secret"));
}
