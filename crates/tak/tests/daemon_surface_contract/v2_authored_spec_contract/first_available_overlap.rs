use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::json;

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::{run_tak_output, write_tasks};

#[test]
fn first_available_keeps_a_worker_only_in_its_earliest_matching_tier() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    let socket = PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::RemoteSubmissionFlow);
    write_tasks(&workspace, TASKS).unwrap();
    let env = BTreeMap::from([
        ("TAKD_SOCKET".into(), "../takd.sock".into()),
        ("XDG_STATE_HOME".into(), "../state".into()),
    ]);

    let output = run_tak_output(&workspace, &["run", "//:check"], &env).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let requests = daemon.finish_expecting(7);
    let candidates = &requests[2]["operation"]["run"]["jobs"][0]["placement_candidates"];
    assert_eq!(
        candidates,
        &json!([
            {"node_id": "worker-a", "kind": "remote", "transport": "direct",
             "reason": "healthy protocol-v2 worker", "tier": 0},
            {"node_id": "worker-b", "kind": "remote", "transport": "direct",
             "reason": "healthy protocol-v2 worker", "tier": 0},
            {"node_id": "local", "kind": "local", "transport": null,
             "reason": "local execution", "tier": 2}
        ])
    );
}

const TASKS: &str = r#"POLICY = Execution.FirstAvailable(placements=[
  Execution.Remote(pool="builders", selection=RemoteSelection.Balanced()),
  Execution.Remote(transport=Transport.DirectHttps(), selection=RemoteSelection.Balanced()),
  Execution.Local(),
])
SPEC = module_spec(spec_version=2, defaults=Defaults(execution=POLICY), tasks=[
  task("check", steps=[cmd("true")]),
])
SPEC
"#;
