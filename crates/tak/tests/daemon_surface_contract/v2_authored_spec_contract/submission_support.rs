use std::collections::BTreeMap;
use std::path::Path;

use serde_json::{Value, json};

pub(super) const TASKS: &str = r#"SPEC = module_spec(
  spec_version=2,
  defaults=Defaults(execution=Execution.Local(), pass_env=["DEFAULT_TOKEN"]),
  tasks=[
    task("dep", steps=[cmd("sh", "-c", "echo client > client-executor-ran")]),
    task("target", deps=[":dep"], steps=[cmd("true")], pass_env=["TASK_TOKEN"]),
    task("unrelated", steps=[cmd("false")], pass_env=["MISSING_UNRELATED"]),
  ],
)
SPEC
"#;

pub(super) fn environment(socket: &Path) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("TAKD_SOCKET".into(), socket.display().to_string()),
        ("DEFAULT_TOKEN".into(), "default-secret".into()),
        ("TASK_TOKEN".into(), "task-secret".into()),
        ("CLI_TOKEN".into(), "cli-secret".into()),
    ])
}

pub(super) fn assert_requests(requests: &[Value]) {
    assert_eq!(
        operation_types(requests),
        ["SubmitRun", "UploadWorkspace", "CommitRun", "AttachRun"]
    );
    let submit = &requests[0]["operation"];
    assert_eq!(submit["run"]["targets"], json!(["//:target"]));
    assert_eq!(
        submit["run"]["options"],
        json!({"max_parallel_jobs": 3, "keep_going": true})
    );
    assert_eq!(task_ids(submit), ["//:dep", "//:target"]);
    assert_eq!(submit["run"]["job_edges"].as_array().unwrap().len(), 1);
    assert_eq!(
        environment_names(submit),
        ["CLI_TOKEN", "DEFAULT_TOKEN", "TASK_TOKEN"]
    );
    let jobs = submit["run"]["jobs"].as_array().unwrap();
    assert!(
        jobs.iter()
            .all(|job| !job["placement_candidates"].as_array().unwrap().is_empty())
    );
    assert!(jobs.iter().all(|job| {
        !job["context_manifest"]["paths"]
            .as_array()
            .unwrap()
            .is_empty()
    }));
    assert!(
        submit["run"]["workspace"]["manifest"]["entries"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["path"] == "input.txt")
    );
}

fn operation_types(requests: &[Value]) -> Vec<&str> {
    requests
        .iter()
        .map(|request| request["operation"]["type"].as_str().unwrap())
        .collect()
}

fn task_ids(submit: &Value) -> Vec<&str> {
    submit["run"]["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|task| task["task_id"].as_str().unwrap())
        .collect()
}

fn environment_names(submit: &Value) -> Vec<&str> {
    submit["environment_values"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect()
}
