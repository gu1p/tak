use std::collections::{BTreeMap, HashMap};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use tak_core::model::WorkspaceSpec;
use tak_proto::local_daemon::v2::RunLifecycleState;
use takd::RunStore;

use crate::support::local_daemon::LocalDaemonGuard;
use crate::support::{run_tak_output, tak_bin, write_tasks};

#[test]
fn foreground_reports_every_changed_destination_and_copies_nothing() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    std::fs::write(workspace.join("existing.txt"), "submitted").unwrap();
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let _daemon = LocalDaemonGuard::spawn(&socket, &empty_spec(&workspace));
    let child = Command::new(tak_bin())
        .current_dir(&workspace)
        .args(["run", "//:produce"])
        .env("TAKD_SOCKET", "../d.sock")
        .env("XDG_STATE_HOME", "../state")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let store = RunStore::with_db_path(socket.with_extension("v2.sqlite")).unwrap();
    let run_id = wait_for_running(&store);
    std::fs::write(workspace.join("existing.txt"), "local-existing").unwrap();
    std::fs::write(workspace.join("new.txt"), "local-new").unwrap();
    let output = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success());
    assert!(stderr.contains("existing.txt"), "{stderr}");
    assert!(stderr.contains("new.txt"), "{stderr}");
    assert_eq!(
        std::fs::read(workspace.join("existing.txt")).unwrap(),
        b"local-existing"
    );
    assert_eq!(
        std::fs::read(workspace.join("new.txt")).unwrap(),
        b"local-new"
    );
    assert!(!workspace.join("safe.txt").exists());
    let destination = temp.path().join("retrieved");
    let environment = BTreeMap::from([
        ("TAKD_SOCKET".into(), "../d.sock".into()),
        ("XDG_STATE_HOME".into(), "../state".into()),
    ]);
    let destination_text = destination.to_str().unwrap();
    let args = ["runs", "outputs", &run_id, "--to", destination_text];
    let retrieved = run_tak_output(&workspace, &args, &environment).unwrap();
    assert!(retrieved.status.success());
    assert_eq!(
        std::fs::read(destination.join("safe.txt")).unwrap(),
        b"daemon-safe"
    );
}

fn wait_for_running(store: &RunStore) -> String {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(run) = store.list_runs().unwrap().first()
            && run.state == RunLifecycleState::Running
        {
            return run.run_id.clone();
        }
        assert!(Instant::now() < deadline, "run never reached running");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn empty_spec(root: &std::path::Path) -> WorkspaceSpec {
    WorkspaceSpec {
        project_id: "v2-output-conflict".into(),
        root: root.to_path_buf(),
        tasks: BTreeMap::new(),
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    }
}

const TASKS: &str = r#"SPEC = module_spec(spec_version=2, tasks=[task(
  "produce",
  outputs=[path("existing.txt"), path("new.txt"), path("safe.txt")],
  steps=[cmd("sh", "-c", "sleep 1; printf daemon-existing > existing.txt; printf daemon-new > new.txt; printf daemon-safe > safe.txt")],
)])
SPEC
"#;
