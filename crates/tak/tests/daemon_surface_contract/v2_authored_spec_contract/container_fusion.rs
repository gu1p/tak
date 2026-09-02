use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde_json::json;

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::{run_tak_output, write_tasks};

#[test]
fn container_session_cascade_submits_dependency_closure_as_one_job() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    let socket = PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::SubmissionFlow);
    let environment = BTreeMap::from([("TAKD_SOCKET".into(), "../d.sock".into())]);

    let output = run_tak_output(&workspace, &["run", "//:target"], &environment).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let requests = daemon.finish_expecting(5);
    let run = &requests[0]["operation"]["run"];
    assert_eq!(run["tasks"].as_array().unwrap().len(), 2);
    assert_eq!(run["jobs"].as_array().unwrap().len(), 1);
    assert_eq!(run["job_edges"], json!([]));
    let job = &run["jobs"][0];
    assert_eq!(job["task_ids"], json!(["//:dep", "//:target"]));
    assert_eq!(job["session"]["reuse"], json!({"kind": "container"}));
    for task in run["tasks"].as_array().unwrap() {
        assert_eq!(task["job_id"], job["job_id"]);
        assert_eq!(task["steps"].as_array().unwrap().len(), 1);
    }
    assert_eq!(run["tasks"][1]["dependencies"], json!(["//:dep"]));
}

const TASKS: &str = r#"BUILD = session(
  "build",
  execution=Execution.Local(),
  reuse=SessionReuse.Container(),
)
SPEC = module_spec(
  spec_version=2,
  tasks=[
    task("dep", steps=[cmd("sh", "-c", "echo dep")]),
    task("target", deps=[":dep"], steps=[cmd("sh", "-c", "echo target")],
         use_session=BUILD, cascade_session=True),
  ],
)
SPEC
"#;
