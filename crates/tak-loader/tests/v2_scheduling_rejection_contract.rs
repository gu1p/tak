use std::fs;

use serde_json::json;
use tak_loader::{LoadOptions, inspect_authored_root_module};

#[test]
fn v2_preserves_existing_retry_and_limiter_options() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    fs::write(
        temp.path().join("TASKS.py"),
        r#"SPEC = module_spec(spec_version=2,
  limiters=[resource("ram", 4, unit="gib"),
            process_cap("sim", 2, match="simulator")],
  tasks=[task("check", retry=retry(attempts=3, on_exit=[7],
    backoff=exp_jitter(min_s=0.25, max_s=4, jitter="full")))])
SPEC
"#,
    )
    .unwrap();
    let root = inspect_authored_root_module(temp.path(), &LoadOptions::default()).unwrap();
    let module = serde_json::to_value(root.module).unwrap();
    assert_eq!(module["tasks"][0]["retry"]["on_exit"], json!([7]));
    assert_eq!(module["tasks"][0]["retry"]["jitter"], "full");
    assert_eq!(module["tasks"][0]["retry"]["backoff_millis"], 250);
    assert_eq!(module["tasks"][0]["retry"]["max_backoff_millis"], 4000);
    assert_eq!(module["limiter_definitions"][0]["unit"], "gib");
    assert_eq!(
        module["limiter_definitions"][1]["match_pattern"],
        "simulator"
    );
}

#[test]
fn v2_queue_max_pending_has_actionable_removal_guidance() {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    fs::write(
        temp.path().join("TASKS.py"),
        r#"SPEC = module_spec(spec_version=2,
  queues=[queue_def("build", slots=2, max_pending=4)],
  tasks=[task("check", queue=queue_use("build"))])
SPEC
"#,
    )
    .unwrap();

    let error = inspect_authored_root_module(temp.path(), &LoadOptions::default())
        .expect_err("removed max_pending must fail")
        .to_string();
    assert!(
        error.contains("`max_pending` was removed in spec_version=2"),
        "{error}"
    );
    assert!(error.contains("use `slots`"), "{error}");
}
