use std::collections::BTreeMap;
use std::fs;

use crate::support::{run_tak_output, write_tasks};

#[test]
fn mixed_limiter_hold_modes_fail_before_daemon_submission() {
    let error = run_error(
        r#"SPEC = module_spec(
  spec_version=2,
  limiters=[resource("cpu", 2, scope=Scope.Project)],
  tasks=[
    task("dep", needs=[need("cpu", scope=Scope.Project, hold=Hold.During)]),
    task("target", deps=[":dep"],
         needs=[need("cpu", scope=Scope.Project, hold=Hold.AtStart)]),
  ],
)
SPEC
"#,
    );
    assert!(error.contains("cannot mix hold modes"), "{error}");
}

#[test]
fn mismatched_scoped_queue_reference_fails_before_daemon_submission() {
    let error = run_error(
        r#"SPEC = module_spec(
  spec_version=2,
  queues=[queue_def("build", slots=1, scope=Scope.Project)],
  tasks=[task("target", queue=queue_use("build", scope=Scope.Machine))],
)
SPEC
"#,
    );
    assert!(error.contains("unknown scoped queue `build`"), "{error}");
}

fn run_error(source: &str) -> String {
    fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, source).unwrap();
    let environment = BTreeMap::from([("TAKD_SOCKET".into(), "missing.sock".into())]);
    let output = run_tak_output(&workspace, &["run", "//:target"], &environment).unwrap();
    assert!(!output.status.success());
    String::from_utf8_lossy(&output.stderr).into_owned()
}
