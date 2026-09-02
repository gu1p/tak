use std::collections::BTreeMap;

use serde_json::json;

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::{run_tak_output, write_tasks};

#[test]
fn v2_container_runtime_and_timeout_are_submitted_to_takd() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::RemoteSubmissionFlow);
    let environment = BTreeMap::from([("TAKD_SOCKET".into(), "../takd.sock".into())]);

    let output = run_tak_output(&workspace, &["run", "//:check"], &environment).unwrap();
    assert!(
        output.status.success(),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let requests = daemon.finish_expecting(5);
    let task = &requests[0]["operation"]["run"]["tasks"][0];
    assert_eq!(task["timeout_s"], 7);
    assert_eq!(
        task["runtime"],
        json!({"kind":"container","source":{"kind":"image","image":"alpine:3.20"},
            "resources":{"cpu_millis":1500,"memory_bytes":268435456}})
    );
    assert_eq!(
        requests[0]["operation"]["run"]["jobs"][0]["resources"],
        json!({"cpu_millis":1500,"memory_bytes":268435456,"execution_slots":1})
    );
}

const TASKS: &str = r#"RUNTIME = Container.Image(
  "alpine:3.20", resources=Container.Resources(cpu_cores=1.5, memory_mb=256),
)
SPEC = module_spec(spec_version=2, tasks=[task(
  "check", steps=[cmd("true")], timeout_s=7,
  execution=Execution.Local(container=RUNTIME),
)])
SPEC
"#;
