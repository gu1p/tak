use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde_json::json;

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::{run_tak_output, write_tasks};

#[test]
fn authored_token_bucket_reaches_submit_run_with_exact_fixed_point_values() {
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
    let requests = daemon.finish_expecting(4);
    let run = &requests[0]["operation"]["run"];
    assert_eq!(
        run["limiter_definitions"][0],
        json!({"kind": "rate_limit", "name": "api", "scope": "project",
               "scope_key": null, "burst": 2,
               "refill_millis_per_second": 2500})
    );
    assert_eq!(
        run["jobs"][0]["limiter_claims"],
        json!([{"name": "api", "amount_millis": 250}])
    );
}

const TASKS: &str = r#"SPEC = module_spec(
  spec_version=2,
  limiters=[rate_limit("api", burst=2, refill_per_second=2.5,
                       scope=Scope.Project)],
  tasks=[task("target", needs=[need("api", slots=.25,
    scope=Scope.Project, hold=Hold.AtStart)])],
)
SPEC
"#;
