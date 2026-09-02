use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::json;

use super::super::runs_cli_contract::fake_daemon::{FakeRunDaemon, Reply};
use crate::support::{run_tak_output, write_tasks};

#[test]
fn remote_v2_resolves_daemon_candidates_before_submitting_concrete_work() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    let socket = PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let daemon = FakeRunDaemon::spawn(&socket, Reply::RemoteSubmissionFlow);
    let env = BTreeMap::from([
        ("TAKD_SOCKET".into(), "../d.sock".into()),
        ("XDG_STATE_HOME".into(), "../state".into()),
        ("BUILD_TOKEN".into(), "do-not-render-build".into()),
    ]);
    write_tasks(&workspace, TASKS).unwrap();

    let output = run_tak_output(&workspace, &["run", "//:check"], &env).unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");
    let requests = daemon.finish_expecting(6);
    assert!(stdout.contains("worker-a"), "{stdout}");
    assert_eq!(requests[0]["operation"]["type"], "ResolveRemoteCandidates");
    assert_eq!(
        requests[0]["operation"]["requirements"],
        json!({
            "pool": "build", "required_tags": ["builder"],
            "required_capabilities": ["linux"], "transport": "direct"
        })
    );
    let jobs = requests[1]["operation"]["run"]["jobs"].as_array().unwrap();
    assert_eq!(jobs.len(), 2);
    let policy_id = jobs[0]["placement_policy"]["policy_id"].as_str().unwrap();
    assert!(policy_id.starts_with("remote-balanced-"), "{policy_id}");
    for job in jobs {
        assert_eq!(job["placement_policy"]["policy_id"], policy_id);
        assert_eq!(job["placement_policy"]["selection"], "balanced");
        assert_eq!(job["placement_candidates"][0]["node_id"], "worker-a");
    }
    assert!(!stdout.contains("do-not-render-build") && !stderr.contains("do-not-render-build"));
}

const TASKS: &str = r#"REMOTE = Execution.Remote(pool="build",
  required_tags=["builder"], required_capabilities=["linux"],
  transport=Transport.DirectHttps(), selection=RemoteSelection.Balanced())
SPEC = module_spec(spec_version=2, defaults=Defaults(
  pass_env=["BUILD_TOKEN"], execution=REMOTE), tasks=[
  task("dep", steps=[cmd("true")]),
  task("check", deps=[":dep"], steps=[cmd("true")])])
SPEC
"#;
