use std::fs;

use serde_json::json;
use tak_loader::{LoadOptions, inspect_authored_root_module};

#[test]
fn decide_evaluates_python_in_tak_and_keeps_its_diagnostic_reason() {
    let execution = load_execution(
        r#"POLICY_CONTEXT = PolicyContext(local_cpu_percent=92.5)
def choose(ctx):
  if ctx["local"]["cpu_percent"] > 90:
    return Decision.remote(reason="LOCAL_CPU_HIGH", required_tags=["arm"])
  return Decision.local(reason="LOCAL_IDLE")
SPEC=module_spec(spec_version=2, tasks=[task("check", execution=Execution.Decide(choose))])
SPEC
"#,
    );

    assert_eq!(execution["kind"], "remote_only");
    assert_eq!(execution["remote"]["required_tags"], json!(["arm"]));
    assert_eq!(execution["remote"]["reason"], "LOCAL_CPU_HIGH");
}

#[test]
fn first_available_preserves_concrete_authored_order() {
    let execution = load_execution(
        r#"SPEC=module_spec(spec_version=2, tasks=[task("check", execution=
  Execution.FirstAvailable([Execution.Remote(pool="build"), Execution.Local()]))])
SPEC
"#,
    );

    assert_eq!(execution["kind"], "first_available");
    assert_eq!(execution["placements"][0]["kind"], "remote_only");
    assert_eq!(execution["placements"][1]["kind"], "local_only");
}

fn load_execution(source: &str) -> serde_json::Value {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    fs::write(temp.path().join("TASKS.py"), source).unwrap();
    let root = inspect_authored_root_module(temp.path(), &LoadOptions::default()).unwrap();
    serde_json::to_value(root.module.tasks[0].execution.as_ref().unwrap()).unwrap()
}
