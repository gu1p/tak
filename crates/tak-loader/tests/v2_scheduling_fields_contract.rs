use std::fs;

use serde_json::json;
use tak_loader::{AuthoredRootModule, LoadOptions, inspect_authored_root_module};

#[test]
fn v2_loader_preserves_scheduling_fields_and_direct_session_use() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    fs::write(
        temp.path().join("TASKS.py"),
        r#"S = session("s", execution=Execution.Local(),
  affinity=Affinity.PreferSameNode("s"))
SPEC = module_spec(
  spec_version=2,
  limiters=[resource("cpu", 2, scope=Scope.Project)],
  queues=[queue_def("build", slots=2, scope=Scope.Project)],
  defaults=Defaults(retry=retry(attempts=3, backoff=fixed(0.25)),
                    queue=queue_use("build", scope=Scope.Project)),
  tasks=[task("check", use_session=S,
    needs=[need("cpu", slots=1.5, scope=Scope.Project, hold=Hold.During)])],
)
SPEC
"#,
    )
    .unwrap();

    let AuthoredRootModule::V2(root) =
        inspect_authored_root_module(temp.path(), &LoadOptions::default()).unwrap()
    else {
        panic!("expected v2 root")
    };
    let module = serde_json::to_value(root.module).unwrap();
    assert_eq!(module["defaults"]["retry"]["max_attempts"], 3);
    assert_eq!(
        module["defaults"]["queue"],
        json!({"name": "build", "scope": "project", "slots": 1, "priority": 0})
    );
    assert_eq!(module["queue_definitions"][0]["max_parallel_tasks"], 2);
    assert_eq!(module["limiter_definitions"][0]["capacity_millis"], 2000);
    assert_eq!(module["limiter_definitions"][0]["scope"], "project");
    let task = &module["tasks"][0];
    assert_eq!(task["session"]["name"], "s");
    assert_eq!(
        task["limiter_claims"][0],
        json!({"name": "cpu", "scope": "project", "amount_millis": 1500,
               "hold": "during"})
    );
}
