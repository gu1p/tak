use std::collections::BTreeMap;

use serde_json::json;

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::{run_tak_output, write_tasks};

#[test]
fn v2_run_local_override_is_resolved_before_daemon_submission() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(
        &workspace,
        r#"SPEC=module_spec(spec_version=2, tasks=[task("check",
  steps=[cmd("true")], execution=Execution.Remote(
    container=Container.Image("alpine:3.20")))])
SPEC
"#,
    )
    .unwrap();
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::SubmissionFlow);
    let environment = BTreeMap::from([("TAKD_SOCKET".into(), "../takd.sock".into())]);

    let output = run_tak_output(&workspace, &["run", "--local", "//:check"], &environment).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let requests = daemon.finish_expecting(5);
    let run = &requests[0]["operation"]["run"];
    assert_eq!(run["jobs"][0]["placement_candidates"][0]["kind"], "local");
    assert_eq!(
        run["tasks"][0]["runtime"],
        json!({"kind":"container","source":{"kind":"image","image":"alpine:3.20"}})
    );
}
