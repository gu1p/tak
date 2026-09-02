use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::{run_tak_output, write_tasks};

#[test]
fn worktree_scheduling_submits_one_stable_opaque_owner_key() {
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
    let queue = &run["queue_definitions"][0];
    let limiter = &run["limiter_definitions"][0];
    assert_eq!(queue["scope"], "worktree");
    assert_eq!(limiter["scope"], "worktree");
    let key = queue["scope_key"].as_str().unwrap();
    assert_eq!(limiter["scope_key"], key);
    assert!(key.starts_with("worktree-") && key.len() == 73, "{key}");
}

const TASKS: &str = r#"SPEC = module_spec(
  spec_version=2,
  queues=[queue_def("build", slots=1, scope=Scope.Worktree)],
  limiters=[resource("cpu", 1, scope=Scope.Worktree)],
  defaults=Defaults(queue=queue_use("build", scope=Scope.Worktree)),
  tasks=[task("target", needs=[need("cpu", scope=Scope.Worktree)])],
)
SPEC
"#;
