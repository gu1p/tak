use std::collections::{BTreeMap, HashMap};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use tak_core::model::WorkspaceSpec;
use takd::RunStore;

use crate::support::local_daemon::LocalDaemonGuard;
use crate::support::{run_tak_output, tak_bin, write_tasks};

#[test]
fn reattach_materializes_into_the_original_checkout_after_client_loss() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let checkout = temp.path().join("checkout");
    let elsewhere = temp.path().join("elsewhere");
    write_tasks(&checkout, TASKS).unwrap();
    std::fs::create_dir(&elsewhere).unwrap();
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let _daemon = LocalDaemonGuard::spawn(&socket, &empty_spec(&checkout));
    let mut child = Command::new(tak_bin())
        .current_dir(&checkout)
        .args(["run", "//:produce"])
        .env("TAKD_SOCKET", "../d.sock")
        .env("XDG_STATE_HOME", "../state")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let store = RunStore::with_db_path(socket.with_extension("v2.sqlite")).unwrap();
    let run_id = wait_for_run(&store, |state| state == "running");
    child.kill().unwrap();
    child.wait().unwrap();
    wait_for_run(&store, |state| state == "succeeded");
    assert!(!checkout.join("dist/result.txt").exists());
    let environment = BTreeMap::from([
        ("TAKD_SOCKET".into(), "../d.sock".into()),
        ("XDG_STATE_HOME".into(), "../state".into()),
    ]);
    let output = run_tak_output(&elsewhere, &["runs", "attach", &run_id], &environment).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read(checkout.join("dist/result.txt")).unwrap(),
        b"daemon"
    );
    assert!(!elsewhere.join("dist/result.txt").exists());
}

fn wait_for_run(store: &RunStore, predicate: impl Fn(&str) -> bool) -> String {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(run) = store.list_runs().unwrap().first()
            && predicate(run.state.as_str())
        {
            return run.run_id.clone();
        }
        assert!(
            Instant::now() < deadline,
            "run did not reach expected state"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn empty_spec(root: &std::path::Path) -> WorkspaceSpec {
    WorkspaceSpec {
        project_id: "v2-output-reattach".into(),
        root: root.to_path_buf(),
        tasks: BTreeMap::new(),
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    }
}

const TASKS: &str = r#"SPEC = module_spec(spec_version=2, tasks=[task(
  "produce", outputs=[path("dist/result.txt")],
  steps=[cmd("sh", "-c", "sleep 1; mkdir -p dist; printf daemon > dist/result.txt")],
)])
SPEC
"#;
