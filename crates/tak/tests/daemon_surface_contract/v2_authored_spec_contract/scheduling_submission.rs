use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde_json::json;

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::{run_tak_output, write_tasks};

#[test]
fn authored_v2_scheduling_and_session_fields_reach_the_daemon() {
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
    if !output.status.success() {
        daemon.finish_expecting(0);
        panic!("{}", String::from_utf8_lossy(&output.stderr));
    }
    let requests = daemon.finish_expecting(5);
    let run = &requests[0]["operation"]["run"];
    let target = run["jobs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|job| job["task_ids"] == json!(["//:target"]))
        .unwrap();
    assert_eq!(
        target["retry"],
        json!({"max_attempts": 3, "on_exit": [], "backoff_millis": 250,
               "max_backoff_millis": 250, "jitter": "none"})
    );
    assert_eq!(target["queue"], "build");
    assert_eq!(target["queue_slots"], 2);
    assert_eq!(target["queue_priority"], 100);
    assert_eq!(
        target["limiter_claims"],
        json!([{"name": "cpu", "amount_millis": 1500}])
    );
    assert_eq!(
        target["affinity"],
        json!({"kind": "prefer_same_node", "group": "shared"})
    );
    let target_task = run["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|task| task["task_id"] == "//:target")
        .unwrap();
    assert_eq!(target_task["affinity"], target["affinity"]);
    assert_eq!(target["session"]["name"], "shared");
    assert_eq!(target["session"]["reuse"], json!({"kind": "workspace"}));
    assert_eq!(
        run["queue_definitions"][0],
        json!({"name": "build", "scope": "project", "scope_key": null,
               "max_parallel_tasks": 2, "discipline": "priority"})
    );
    assert_eq!(
        run["limiter_definitions"][0],
        json!({"kind": "resource", "name": "cpu", "scope": "project",
               "scope_key": null, "capacity_millis": 2000, "unit": null,
               "hold": "during"})
    );
}

const TASKS: &str = r#"SHARED = session(
  "shared",
  execution=Execution.Local(),
  reuse=SessionReuse.Workspace(),
  affinity=Affinity.PreferSameNode("shared"),
)
SPEC = module_spec(
  spec_version=2,
  limiters=[resource("cpu", 2, scope=Scope.Project)],
  queues=[queue_def("build", slots=2, discipline=QueueDiscipline.Priority,
                    scope=Scope.Project)],
  defaults=Defaults(
    execution=Execution.Local(),
    retry=retry(attempts=3, backoff=fixed(0.25)),
    queue=queue_use("build", scope=Scope.Project, slots=2, priority=100),
  ),
  tasks=[
    task("dep", steps=[cmd("true")]),
    task("target", deps=[":dep"], steps=[cmd("true")], use_session=SHARED,
         needs=[need("cpu", slots=1.5, scope=Scope.Project, hold=Hold.During)]),
  ],
)
SPEC
"#;
