use std::collections::{BTreeMap, HashMap};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use tak_core::model::WorkspaceSpec;
use tak_proto::local_daemon::v2::RunLifecycleState;
use takd::RunStore;

use crate::support::local_daemon::LocalDaemonGuard;
use crate::support::{tak_bin, write_tasks};

pub(super) fn assert_replacement_preserves_checkout() {
    std::fs::create_dir_all(".tmp").unwrap();
    let temp = tempfile::tempdir_in(".tmp").unwrap();
    let workspace = temp.path().join("workspace");
    write_tasks(&workspace, TASKS).unwrap();
    for root in ["file-dist", "link-dist"] {
        std::fs::create_dir_all(workspace.join(root).join("nested")).unwrap();
        std::fs::write(workspace.join(root).join("one.txt"), "submitted-one").unwrap();
        std::fs::write(workspace.join(root).join("nested/two.txt"), "submitted-two").unwrap();
    }
    let socket = std::path::PathBuf::from(".tmp")
        .join(temp.path().file_name().unwrap())
        .join("d.sock");
    let daemon = LocalDaemonGuard::spawn(&socket, &empty_spec(&workspace));
    let child = Command::new(tak_bin())
        .current_dir(&workspace)
        .args(["run", "//:produce"])
        .env("TAKD_SOCKET", "../d.sock")
        .env("XDG_STATE_HOME", "../state")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let store = RunStore::with_db_path(daemon.db_path().to_path_buf()).unwrap();
    wait_for_running(&store);
    for root in ["file-dist", "link-dist"] {
        std::fs::write(workspace.join(root).join("one.txt"), "local-one").unwrap();
        std::fs::write(workspace.join(root).join("nested/two.txt"), "local-two").unwrap();
        std::fs::write(workspace.join(root).join("local.txt"), "local-new").unwrap();
    }

    let output = child.wait_with_output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "stderr:\n{stderr}");
    for root in ["file-dist", "link-dist"] {
        for (relative, expected) in [
            ("one.txt", b"local-one".as_slice()),
            ("nested/two.txt", b"local-two".as_slice()),
            ("local.txt", b"local-new".as_slice()),
        ] {
            let path = format!("{root}/{relative}");
            assert!(stderr.contains(&path), "missing {path}:\n{stderr}");
            assert_eq!(std::fs::read(workspace.join(path)).unwrap(), expected);
        }
    }
}

fn wait_for_running(store: &RunStore) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if store
            .list_runs()
            .unwrap()
            .first()
            .is_some_and(|run| run.state == RunLifecycleState::Running)
        {
            return;
        }
        assert!(Instant::now() < deadline, "run never reached running");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn empty_spec(root: &std::path::Path) -> WorkspaceSpec {
    WorkspaceSpec {
        project_id: "v2-output-replacement-conflict".into(),
        root: root.to_path_buf(),
        tasks: BTreeMap::new(),
        sessions: BTreeMap::new(),
        limiters: HashMap::new(),
        queues: HashMap::new(),
    }
}

const TASKS: &str = r#"SPEC = module_spec(spec_version=2, tasks=[task(
  "produce", outputs=[path("file-dist"), path("link-dist")],
  steps=[cmd("sh", "-c", "sleep 1; rm -rf file-dist link-dist; printf daemon > file-dist; ln -s target.txt link-dist")],
)])
SPEC
"#;
