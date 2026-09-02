use std::collections::BTreeMap;
use std::path::PathBuf;

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::{run_tak_output, write_tasks};

#[test]
fn remote_only_still_rejects_an_empty_candidate_set() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    let socket = PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("takd.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::EmptyRemoteSubmissionFlow);
    write_tasks(&workspace, TASKS).unwrap();
    let env = BTreeMap::from([
        ("TAKD_SOCKET".into(), "../takd.sock".into()),
        ("XDG_STATE_HOME".into(), "../state".into()),
    ]);

    let output = run_tak_output(&workspace, &["run", "//:check"], &env).unwrap();
    daemon.finish_expecting(1);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(
        stderr.contains("no connected protocol-v2 worker"),
        "{stderr}"
    );
}

const TASKS: &str = r#"SPEC = module_spec(spec_version=2,
  defaults=Defaults(execution=Execution.Remote()),
  tasks=[task("check", steps=[cmd("true")])])
SPEC
"#;
